/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate gleam;
extern crate glutin;
extern crate webrender;
extern crate webrender_traits;

use gleam::gl;
use std::env;
use std::path::PathBuf;
use webrender_traits::{ClipId, ClipRegion, ColorF, Epoch};
use webrender_traits::{DeviceUintSize, LayoutPoint, LayoutRect, LayoutSize};
use webrender_traits::{PipelineId, RenderApi, TransformStyle};

struct ScrollState {
    root_clip_id: ClipId,
    root_scroller_pos: LayoutPoint,
    nested_clip_id: ClipId,
    nested_scroller_pos: LayoutPoint,
}

impl ScrollState {
    pub fn new(root: ClipId, nested: ClipId) -> ScrollState {
        ScrollState {
            root_clip_id: root,
            root_scroller_pos: LayoutPoint::zero(),
            nested_clip_id: nested,
            nested_scroller_pos: LayoutPoint::zero(),
        }
    }

    pub fn scroll(&mut self, root: bool, delta: (i32, i32), api: &RenderApi) {
        if root {
            self.root_scroller_pos.x += delta.0 as f32;
            self.root_scroller_pos.y += delta.1 as f32;
            api.scroll_node_with_id(self.root_scroller_pos, self.root_clip_id);
        } else {
            self.nested_scroller_pos.x += delta.0 as f32;
            self.nested_scroller_pos.y += delta.1 as f32;
            api.scroll_node_with_id(self.nested_scroller_pos, self.nested_clip_id);
        }
    }
}

struct Notifier {
    window_proxy: glutin::WindowProxy,
}

impl Notifier {
    fn new(window_proxy: glutin::WindowProxy) -> Notifier {
        Notifier {
            window_proxy: window_proxy,
        }
    }
}

impl webrender_traits::RenderNotifier for Notifier {
    fn new_frame_ready(&mut self) {
        #[cfg(not(target_os = "android"))]
        self.window_proxy.wakeup_event_loop();
    }

    fn new_scroll_frame_ready(&mut self, _composite_needed: bool) {
        #[cfg(not(target_os = "android"))]
        self.window_proxy.wakeup_event_loop();
    }
}

trait HandyDandyRectBuilder {
    fn to(&self, x2: i32, y2: i32) -> LayoutRect;
}
// Allows doing `(x, y).to(x2, y2)` to build a LayoutRect
impl HandyDandyRectBuilder for (i32, i32) {
    fn to(&self, x2: i32, y2: i32) -> LayoutRect {
        LayoutRect::new(LayoutPoint::new(self.0 as f32, self.1 as f32),
                        LayoutSize::new((x2 - self.0) as f32, (y2 - self.1) as f32))
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();
    let res_path = if args.len() > 1 {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    };

    let window = glutin::WindowBuilder::new()
                .with_title("WebRender Scrolling Sample")
                .with_gl(glutin::GlRequest::GlThenGles {
                    opengl_version: (3, 2),
                    opengles_version: (3, 0)
                })
                .build()
                .unwrap();

    unsafe {
        window.make_current().ok();
    }

    let gl = match gl::GlType::default() {
        gl::GlType::Gl => unsafe { gl::GlFns::load_with(|symbol| window.get_proc_address(symbol) as *const _) },
        gl::GlType::Gles => unsafe { gl::GlesFns::load_with(|symbol| window.get_proc_address(symbol) as *const _) },
    };

    println!("OpenGL version {}", gl.get_string(gl::VERSION));
    println!("Shader resource path: {:?}", res_path);

    let (width, height) = window.get_inner_size_pixels().unwrap();

    let opts = webrender::RendererOptions {
        resource_override_path: res_path,
        debug: true,
        precache_shaders: true,
        device_pixel_ratio: window.hidpi_factor(),
        .. Default::default()
    };

    let size = DeviceUintSize::new(width, height);
    let (mut renderer, sender) = webrender::renderer::Renderer::new(gl, opts, size).unwrap();
    let api = sender.create_api();

    let notifier = Box::new(Notifier::new(window.create_window_proxy()));
    renderer.set_render_notifier(notifier);

    let epoch = Epoch(0);
    let root_background_color = ColorF::new(0.3, 0.0, 0.0, 1.0);

    let pipeline_id = PipelineId(0, 0);
    let mut builder = webrender_traits::DisplayListBuilder::new(pipeline_id);

    let bounds = LayoutRect::new(LayoutPoint::zero(), LayoutSize::new(width as f32, height as f32));
    builder.push_stacking_context(webrender_traits::ScrollPolicy::Scrollable,
                                  bounds,
                                  0,
                                  None,
                                  TransformStyle::Flat,
                                  None,
                                  webrender_traits::MixBlendMode::Normal,
                                  Vec::new());

    if true {   // scrolling and clips stuff
        // let's make a scrollbox
        let scrollbox = (0, 0).to(300, 400);
        builder.push_stacking_context(webrender_traits::ScrollPolicy::Scrollable,
                                      LayoutRect::new(LayoutPoint::new(10.0, 10.0),
                                                      LayoutSize::zero()),
                                      0,
                                      None,
                                      TransformStyle::Flat,
                                      None,
                                      webrender_traits::MixBlendMode::Normal,
                                      Vec::new());
        // set the scrolling clip
        let clip_id = builder.define_clip((0, 0).to(1000, 1000),
                                          ClipRegion::simple(&scrollbox),
                                          Some(ClipId::new(42, pipeline_id)));
        builder.push_clip_id(clip_id);
        // now put some content into it.
        // start with a white background
        builder.push_rect((0, 0).to(500, 500),
                          ClipRegion::simple(&(0, 0).to(1000, 1000)),
                          ColorF::new(1.0, 1.0, 1.0, 1.0));
        // let's make a 50x50 blue square as a visual reference
        builder.push_rect((0, 0).to(50, 50),
                          ClipRegion::simple(&(0, 0).to(50, 50)),
                          ColorF::new(0.0, 0.0, 1.0, 1.0));
        // and a 50x50 green square next to it with an offset clip
        // to see what that looks like
        builder.push_rect((50, 0).to(100, 50),
                          ClipRegion::simple(&(60, 10).to(110, 60)),
                          ColorF::new(0.0, 1.0, 0.0, 1.0));

        // Below the above rectangles, set up a nested scrollbox. It's still in
        // the same stacking context, so note that the rects passed in need to
        // be relative to the stacking context.
        let nested_clip_id = builder.define_clip((0, 100).to(300, 400),
                                                 ClipRegion::simple(&(0, 100).to(200, 300)),
                                                 Some(ClipId::new(43, pipeline_id)));
        builder.push_clip_id(nested_clip_id);
        // give it a giant gray background just to distinguish it and to easily
        // visually identify the nested scrollbox
        builder.push_rect((-1000, -1000).to(5000, 5000),
                          ClipRegion::simple(&(-1000, -1000).to(5000, 5000)),
                          ColorF::new(0.5, 0.5, 0.5, 1.0));
        // add a teal square to visualize the scrolling/clipping behaviour
        // as you scroll the nested scrollbox with WASD keys
        builder.push_rect((0, 100).to(50, 150),
                          ClipRegion::simple(&(0, 100).to(50, 150)),
                          ColorF::new(0.0, 1.0, 1.0, 1.0));
        // just for good measure add another teal square in the bottom-right
        // corner of the nested scrollframe content, which can be scrolled into
        // view by the user
        builder.push_rect((250, 350).to(300, 400),
                          ClipRegion::simple(&(250, 350).to(300, 400)),
                          ColorF::new(0.0, 1.0, 1.0, 1.0));
        builder.pop_clip_id(); // nested_clip_id

        builder.pop_clip_id(); // clip_id
        builder.pop_stacking_context();
    }

    builder.pop_stacking_context();

    api.set_display_list(
        Some(root_background_color),
        epoch,
        LayoutSize::new(width as f32, height as f32),
        builder.finalize(),
        true);
    api.set_root_pipeline(pipeline_id);
    api.generate_frame(None);

    let mut scroll_state = ScrollState::new(ClipId::new(42, pipeline_id),
                                            ClipId::new(43, pipeline_id));

    'outer: for event in window.wait_events() {
        let mut events = Vec::new();
        events.push(event);

        for event in window.poll_events() {
            events.push(event);
        }

        for event in events {
            match event {
                glutin::Event::Closed |
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) |
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Q)) => break 'outer,
                // Use up/down/left/right for scrolling the outer scrollbox
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed,
                                             _, Some(glutin::VirtualKeyCode::Down)) => {
                    scroll_state.scroll(true, (0, 10), &api);
                }
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed,
                                             _, Some(glutin::VirtualKeyCode::Up)) => {
                    scroll_state.scroll(true, (0, -10), &api);
                }
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed,
                                             _, Some(glutin::VirtualKeyCode::Left)) => {
                    scroll_state.scroll(true, (-10, 0), &api);
                }
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed,
                                             _, Some(glutin::VirtualKeyCode::Right)) => {
                    scroll_state.scroll(true, (10, 0), &api);
                }
                // Use WASD for scrolling the nested scrollbox
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed,
                                             _, Some(glutin::VirtualKeyCode::S)) => {
                    scroll_state.scroll(false, (0, 10), &api);
                }
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed,
                                             _, Some(glutin::VirtualKeyCode::W)) => {
                    scroll_state.scroll(false, (0, -10), &api);
                }
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed,
                                             _, Some(glutin::VirtualKeyCode::A)) => {
                    scroll_state.scroll(false, (-10, 0), &api);
                }
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed,
                                             _, Some(glutin::VirtualKeyCode::D)) => {
                    scroll_state.scroll(false, (10, 0), &api);
                }
                _ => ()
            }
        }

        renderer.update();
        renderer.render(DeviceUintSize::new(width, height));
        window.swap_buffers().ok();
    }
}