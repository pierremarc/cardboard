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
mod ui_sdl;

use bbox::BBox;
use camera::Camera;
use data::load_geojson;
use lingua::get_planes;
use lingua::get_properties;
use std::env;
use style::{load_style, StyleList};

fn main() {
    let args: Vec<String> = env::args().collect();
    let data_fn = &args[1];
    let style_fn = &args[2];

    // if args.len() == 4 {
    //     let cap_file = &args[3];
    //     Capture::from_records(cap_file).map(|c| replay(&c));
    // }

    let gj = load_geojson(data_fn).unwrap();
    let pl = get_planes(&gj);
    let props = get_properties(&gj);
    let sj = load_style(style_fn).unwrap();
    println!("{:?}", sj);
    let mut style = StyleList::from_config(&sj);
    style.apply(&props);

    println!("N {}", pl.len());

    let bbox = BBox::from_planes(&pl);
    let center = bbox.center();
    let initial_camera = Camera {
        eye: bbox.top_left_near(),
        target: center,
    };
    let camera = initial_camera;

    let mut ui = ui_sdl::UiSdl::new(800, 800);

    ui.run(&pl, &style, initial_camera);
}
