use cairo::Context;
use camera::Camera;
use capture::Capture;
use draw::draw_planes;
use handlers::handle_key_event;
use handlers::handle_key_event_pre;
use handlers::handle_motion_event;
use handlers::handle_wheel_event;
use handlers::PreAction;
use lingua::PlaneList;
use operation::{OpList, Operation};
use sdl2::event::Event;
use sdl2::render::Texture;
use style::StyleList;
use surface_data::create_for_data_unsafe;
use time::PreciseTime;

pub struct UiSdl {
    width: u32,
    height: u32,
    follow_mode: bool,
    capture: Capture,
}

enum PostEventAction {
    Quit,
    Idle,
    Draw(Camera, u32),
}

impl UiSdl {
    pub fn new(width: u32, height: u32) -> UiSdl {
        UiSdl {
            width,
            height,
            follow_mode: false,
            capture: Capture::new(),
        }
    }

    fn update_cam(&self, oc: Option<Camera>, timestamp: u32) -> PostEventAction {
        println!("update_cam {}", oc.is_some());
        match oc {
            Some(c) => PostEventAction::Draw(c, timestamp),
            None => PostEventAction::Idle,
        }
    }

    fn process_event(
        &mut self,
        event: Event,
        camera: &Camera,
        initial_camera: &Camera,
    ) -> PostEventAction {
        match event {
            sdl2::event::Event::Quit { .. } => PostEventAction::Quit,
            sdl2::event::Event::KeyDown {
                keycode,
                keymod,
                timestamp,
                ..
            } => match handle_key_event_pre(keycode, keymod) {
                Some(PreAction::Reset) => PostEventAction::Draw(initial_camera.clone(), timestamp),
                Some(PreAction::Follow) => {
                    self.follow_mode = !self.follow_mode;
                    PostEventAction::Idle
                }
                Some(PreAction::Capture) => {
                    self.capture.toggle();
                    PostEventAction::Idle
                }
                Some(PreAction::Save) => {
                    self.capture.save("capture.cardboard");
                    PostEventAction::Idle
                }
                None => self.update_cam(handle_key_event(keycode, keymod, &camera), timestamp),
            },
            sdl2::event::Event::MouseMotion {
                xrel,
                yrel,
                timestamp,
                ..
            } => {
                if self.follow_mode {
                    self.update_cam(handle_motion_event(xrel, yrel, &camera), timestamp)
                } else {
                    PostEventAction::Idle
                }
            }
            sdl2::event::Event::MouseWheel { y, timestamp, .. } => {
                self.update_cam(handle_wheel_event(y, &camera), timestamp)
            }

            _ => PostEventAction::Idle,
        }
    }

    pub fn run(&mut self, planes: &PlaneList, style: &StyleList, initial_camera: Camera) {
        let sdl = sdl2::init().unwrap();
        let video_subsystem = sdl.video().unwrap();
        let mut event_pump = sdl.event_pump().unwrap();
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

        match self.paint(
            &draw_planes(planes, &camera, &style, f64::from(self.width)),
            &mut sdl_texture,
            style,
        ) {
            Ok(s) => println!("draw success {}", s),
            Err(e) => println!("draw failure {}", e),
        };
        match canvas.copy(&sdl_texture, None, None) {
            Ok(r) => {
                println!("canvas copied {:?}", r);
                canvas.present();
            }
            Err(e) => {
                println!("canvas.copy error {}", e);
            }
        };

        'main: loop {
            for event in event_pump.wait_iter() {
                match self.process_event(event, &camera, &initial_camera) {
                    PostEventAction::Quit => break 'main,
                    PostEventAction::Draw(new_camera, timestamp) => {
                        camera = new_camera;
                        self.capture.map(timestamp, camera);
                        let start_paint = PreciseTime::now();
                        self.paint(
                            &draw_planes(planes, &camera, &style, f64::from(self.width)),
                            &mut sdl_texture,
                            style,
                        ).and_then(|_| {
                            println!("Painted in {}", start_paint.to(PreciseTime::now()));
                            canvas.copy(&sdl_texture, None, None)
                        }).map(|_| canvas.present());

                        // {
                        //     Ok(s) => println!("draw success {}", s),
                        //     Err(e) => println!("draw failure {}", e),
                        // };
                        // match canvas.copy(&sdl_texture, None, None) {
                        //     Ok(r) => {
                        //         println!("canvas copied {:?}", r);
                        //         canvas.present();
                        //     }
                        //     Err(e) => {
                        //         println!("canvas.copy error {}", e);
                        //     }
                        // };
                    }
                    PostEventAction::Idle => (),
                }

                // std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
    }

    fn paint(
        &self,
        ops: &OpList,
        texture: &mut Texture,
        style: &StyleList,
    ) -> Result<usize, String> {
        let sdl_query = texture.query();
        let rect = sdl2::rect::Rect::new(0, 0, sdl_query.width, sdl_query.height);
        println!("paint {} {}", sdl_query.width, sdl_query.height);

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
                Operation::Line(p) => context.line_to(p.x, p.y),
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
            });
            ops.len()
        })
    }
}
