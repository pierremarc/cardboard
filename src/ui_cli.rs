use cairo::{Context, PDFSurface};
use camera::Camera;
use draw::{get_draw_config, DrawConfig, Drawable};
use lingua::PlaneList;
use operation::paint_op;
use style::{StyleCollection, StyleGetter};

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
        style: &StyleCollection,
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

    fn run_replay(&self, planes: &PlaneList, style: &StyleCollection, target_path: &str) {}

    fn run_print(
        &self,
        planes: &PlaneList,
        style: &StyleCollection,
        camera: Camera,
        target_path: &str,
    ) {
        let surface =
            PDFSurface::create(target_path, f64::from(self.width), f64::from(self.height));
        let context = Context::new(&surface);
        self.paint(
            planes,
            &get_draw_config(planes, &camera, f64::from(self.width)),
            style,
            &context,
        );
    }

    fn paint(
        &self,
        pl: &PlaneList,
        config: &DrawConfig,
        style: &StyleCollection,
        context: &Context,
    ) {
        pl.draw(config, |op| paint_op(&op, style, context));
    }
}
