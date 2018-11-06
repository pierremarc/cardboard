use camera::Camera;
use lingua::Point;
use std::fs::File;
use std::io::prelude::{BufRead, Write};

struct Frame {
    timestamp: u32,
    camera: Camera,
}

fn get_field<T>(i: usize, v: &Vec<&str>) -> Result<T, usize>
where
    T: std::str::FromStr,
{
    match v.get(i) {
        Some(s) => s.parse::<T>().map_err(|_| i),
        None => Err(i),
    }
}

impl Frame {
    fn new(timestamp: u32, camera: Camera) -> Frame {
        Frame { timestamp, camera }
    }

    fn to_record(&self) -> String {
        format!(
            "{} {} {} {} {} {} {}\n",
            self.timestamp,
            self.camera.eye.x,
            self.camera.eye.y,
            self.camera.eye.z,
            self.camera.target.x,
            self.camera.target.y,
            self.camera.target.z,
        )
    }

    fn from_record(r: &str) -> Result<Frame, usize> {
        let fields = r.split(" ").collect();

        let timestamp = get_field::<u32>(0, &fields)?;
        let cam_x = get_field::<f64>(1, &fields)?;
        let cam_y = get_field::<f64>(2, &fields)?;
        let cam_z = get_field::<f64>(3, &fields)?;
        let target_x = get_field::<f64>(4, &fields)?;
        let target_y = get_field::<f64>(5, &fields)?;
        let target_z = get_field::<f64>(6, &fields)?;

        Ok(Frame::new(
            timestamp,
            Camera::new(
                Point::new(cam_x, cam_y, cam_z),
                Point::new(target_x, target_y, target_z),
            ),
        ))
    }
}

pub struct Capture {
    on: bool,
    frames: Vec<Frame>,
}

impl Capture {
    pub fn new() -> Capture {
        Capture {
            on: false,
            frames: Vec::new(),
        }
    }

    pub fn toggle(&mut self) {
        if self.on {
            self.on = false;
        } else {
            self.on = true;
            self.frames = Vec::new();
        }
    }

    pub fn map(&mut self, timetamp: u32, camera: Camera) {
        if self.on {
            self.frames.push(Frame::new(timetamp, camera));
        }
    }

    pub fn save(&self, file_path: &str) -> std::io::Result<()> {
        let mut file = File::create(file_path)?;
        let mut err_count = 0;
        self.frames.iter().for_each(|frame| {
            let rec = frame.to_record();
            match file.write(rec.as_bytes()) {
                Err(_) => err_count += 1,
                _ => (),
            };
        });

        println!("Saved {} with {} errors", file_path, err_count);

        Ok(())
    }

    pub fn from_records(file_path: &str) -> std::io::Result<Capture> {
        let records = std::fs::read_to_string(file_path)?;
        let mut frames: Vec<Frame> = Vec::new();

        records.lines().for_each(|r| {
            Frame::from_record(r).map(|f| frames.push(f));
        });

        Ok(Capture { on: false, frames })
    }
}

fn replay(capture: &Capture) {
    capture
        .frames
        .iter()
        .for_each(|f| println!("R> {}", f.timestamp));
}
