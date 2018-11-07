use cairo::{Context, PDFSurface};
use camera::Camera;
use draw::draw_planes;
use lingua::PlaneList;
use operation::{OpList, Operation};
use std::fs::File;
use style::StyleList;

pub enum CliMode {
    Replay,
    Print,
}

pub struct UiCli {
    width: u32,
    height: u32,
    mode: CliMode,
}

impl UiCli {
    pub fn new(width: u32, height: u32, mode: CliMode) -> UiCli {
        UiCli {
            width,
            height,
            mode,
        }
    }

    pub fn run(
        &self,
        planes: &PlaneList,
        style: &StyleList,
        camera: Option<Camera>,
        target_path: &str,
    ) {
        match self.mode {
            CliMode::Print => match camera {
                Some(cam) => self.run_print(planes, style, cam, target_path),
                None => println!("Camera is missing"),
            },
            CliMode::Replay => self.run_replay(planes, style, target_path),
        }
    }

    fn run_replay(&self, planes: &PlaneList, style: &StyleList, target_path: &str) {}

    fn run_print(&self, planes: &PlaneList, style: &StyleList, camera: Camera, target_path: &str) {
        let surface =
            PDFSurface::create(target_path, f64::from(self.width), f64::from(self.height));
        let context = Context::new(&surface);
        self.paint(
            &draw_planes(planes, &camera, &style, f64::from(self.width)),
            style,
            &context,
        );
    }

    fn paint(&self, ops: &OpList, style: &StyleList, context: &Context) {
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
    }
}
