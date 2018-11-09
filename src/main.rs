extern crate cairo;
extern crate cairo_sys;
extern crate geojson;
extern crate libc;
extern crate nalgebra;
extern crate ordered_float;
extern crate rayon;
extern crate sdl2;
extern crate serde;
extern crate serde_json;
extern crate svgtypes;
extern crate time;

#[macro_use]
extern crate serde_derive;

mod bbox;
mod camera;
mod capture;
mod data;
mod draw;
mod geom;
mod handlers;
mod lingua;
mod operation;
mod style;
mod surface_data;
mod ui_cli;
mod ui_sdl;

use bbox::BBox;
use camera::Camera;
use data::Data;
use lingua::Point;
use std::env;
use std::process;

fn run(args: Vec<String>) {
    let command = &args[1];
    let layers = match Data::from_file(&args[2]) {
        Ok(data) => data,
        Err(e) => {
            println!("{:?}", e);
            process::exit(1)
        }
    };

    println!("N {}", layers.planes.len());

    let bbox = BBox::from_planes(&layers.planes);
    let center = bbox.center();
    let initial_camera = Camera {
        eye: bbox.top_left_near(),
        target: center,
    };

    if "view" == command {
        let mut ui = ui_sdl::UiSdl::new(600, 600);
        ui.run(&layers.planes, &layers.styles, initial_camera);
    } else if "print" == command {
        let width = args[4].parse::<u32>().unwrap_or(595);
        let height = args[5].parse::<u32>().unwrap_or(841);
        let eye_x = args[6].parse::<f64>().unwrap_or(initial_camera.eye.x);
        let eye_y = args[7].parse::<f64>().unwrap_or(initial_camera.eye.y);
        let eye_z = args[8].parse::<f64>().unwrap_or(initial_camera.eye.z);
        let target_x = args[9].parse::<f64>().unwrap_or(initial_camera.target.x);
        let target_y = args[10].parse::<f64>().unwrap_or(initial_camera.target.y);
        let target_z = args[11].parse::<f64>().unwrap_or(initial_camera.target.z);
        let output = &args[12];

        println!(
            "camera {} {} {} {} {} {}",
            eye_x, eye_y, eye_z, target_x, target_y, target_z,
        );
        let ui = ui_cli::UiCli::new(width, height, ui_cli::CliMode::Print);
        ui.run(
            &layers.planes,
            &layers.styles,
            Some(Camera::new(
                Point::new(eye_x, eye_y, eye_z),
                Point::new(target_x, target_y, target_z),
            )),
            output,
        );
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    run(args);
}
