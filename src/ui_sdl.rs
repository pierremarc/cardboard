use cairo::Context;
use camera::Camera;
use capture::Capture;
use draw::draw_planes;
use handlers::handle_key_event;
use handlers::handle_motion_event;
use handlers::handle_wheel_event;
use lingua::PlaneList;
use operation::{OpList, Operation};
use sdl2::render::Canvas;
use sdl2::render::Texture;
use sdl2::video::Window;
use std::rc::Rc;
use style::StyleList;
use surface_data::create_for_data_unsafe;

pub struct UiSdl {
    sdl: sdl2::Sdl,
    width: u32,
    height: u32,
}

impl UiSdl {
    pub fn new(width: u32, height: u32) -> UiSdl {
        let sdl = sdl2::init().unwrap();

        UiSdl { sdl, width, height }
    }

    fn update_cam(
        &self,
        oc: Option<Camera>,
        timestamp: u32,
        dirty: &mut bool,
        camera: &mut Camera,
        capture: &mut Capture,
    ) {
        match oc {
            Some(c) => {
                *camera = c;
                *dirty = true;
                capture.map(timestamp, c);
            }
            None => (),
        };
    }

    pub fn run(&mut self, planes: &PlaneList, style: &StyleList, initial_camera: Camera) {
        let video_subsystem = self.sdl.video().unwrap();
        let mut event_pump = self.sdl.event_pump().unwrap();
        let window = video_subsystem
            .window("Cardoard", self.width, self.height)
            .build()
            .unwrap();
        let mut canvas = sdl2::render::CanvasBuilder::new(window).build().unwrap();
        let mut camera = initial_camera;

        canvas.set_draw_color(sdl2::pixels::Color::RGB(100, 100, 100));
        canvas.clear();
        let texture_creator = canvas.texture_creator();
        let mut sdl_texture: sdl2::render::Texture = texture_creator
            .create_texture(
                Some(sdl2::pixels::PixelFormatEnum::ARGB8888),
                sdl2::render::TextureAccess::Streaming,
                self.width,
                self.height,
            ).unwrap();

        let mut dirty = true;
        let mut key_control = false;
        let control_mod = sdl2::keyboard::LCTRLMOD;

        let mut capture = Capture::new();

        'main: loop {
            for event in event_pump.poll_iter() {
                match event {
                    sdl2::event::Event::Quit { .. } => break 'main,
                    sdl2::event::Event::KeyDown {
                        keycode,
                        keymod,
                        timestamp,
                        ..
                    } => {
                        key_control = keymod.contains(control_mod);
                        self.update_cam(
                            handle_key_event(
                                keycode,
                                keymod,
                                &camera,
                                &initial_camera,
                                &mut capture,
                            ),
                            timestamp,
                            &mut dirty,
                            &mut camera,
                            &mut capture,
                        );
                    }
                    sdl2::event::Event::MouseMotion {
                        xrel,
                        yrel,
                        timestamp,
                        ..
                    } => {
                        if key_control {
                            self.update_cam(
                                handle_motion_event(xrel, yrel, &camera),
                                timestamp,
                                &mut dirty,
                                &mut camera,
                                &mut capture,
                            );
                        }
                    }
                    sdl2::event::Event::MouseWheel { y, timestamp, .. } => {
                        self.update_cam(
                            handle_wheel_event(y, &camera),
                            timestamp,
                            &mut dirty,
                            &mut camera,
                            &mut capture,
                        );
                    }

                    _ => {}
                }

                if dirty {
                    self.paint(
                        &draw_planes(planes, &camera, &style, f64::from(self.width)),
                        &mut sdl_texture,
                        style,
                    );
                    match canvas.copy(&sdl_texture, None, None) {
                        Ok(_) => {
                            canvas.present();
                            dirty = false;
                        }
                        _ => dirty = true,
                    };
                }

                std::thread::sleep(std::time::Duration::from_millis(32));
            }
        }
    }

    fn paint(&self, ops: &OpList, texture: &mut Texture, style: &StyleList) -> Result<(), String> {
        let sdl_query = texture.query();
        let rect = sdl2::rect::Rect::new(0, 0, sdl_query.width, sdl_query.height);
        texture.with_lock(Some(rect), |sdl_data, stride| {
            let surface = create_for_data_unsafe(
                sdl_data,
                cairo::Format::ARgb32,
                sdl_query.width as i32,
                sdl_query.height as i32,
                stride as i32,
            ).unwrap();

            let context = Context::new(&surface);

            context.set_source_rgb(1.0, 1.0, 1.0);
            context.paint();

            ops.iter().for_each(|op| match op {
                Operation::Begin => context.new_path(),
                Operation::Close => context.close_path(),
                Operation::Move(p) => context.move_to(p.x, p.y),
                Operation::Line(p) => context.move_to(p.x, p.y),
                Operation::Paint(i) => style.get_for(i).map_or((), |s| {
                    s.fillColor.map(|color| {
                        context.set_source_rgba(color.red, color.green, color.blue, color.alpha);
                        context.fill_preserve();
                    });

                    s.strokeColor.map(|color| {
                        context.set_line_width(s.strokeWidth);
                        context.set_source_rgba(color.red, color.green, color.blue, color.alpha);
                        context.stroke();
                    });
                }),
            })
        })
    }
}
