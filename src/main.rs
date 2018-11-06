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

mod data;
mod style;
mod surface_data;

use cairo::{Context, Format, ImageSurface};
use data::load_geojson;
use geojson::{Feature, GeoJson, Geometry, PolygonType, Value};
use nalgebra as na;
use nalgebra::distance_squared;
use nalgebra::geometry::Isometry3;
use nalgebra::geometry::{Point2, Point3};
use nalgebra::Vector3;
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use serde_json::{Map, Value as JsonValue};
use std::cmp;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use style::{load_style, Properties, StyleList};
use surface_data::create_for_data_unsafe;
use time::PreciseTime;

pub type Point = Point3<f64>;
pub type Plane = Vec<Point>;
pub type PlaneList = Vec<Plane>;

pub type PropList = Vec<Properties>;

fn plane_from_polygon(poly: PolygonType) -> Plane {
    let exterior_ring = &poly[0];
    exterior_ring
        .iter()
        .map(|pos| Point::new(pos[0], pos[1], pos[2]))
        .collect()
}

fn plane_from_geometry(geom: Geometry, pl: &mut PlaneList) {
    match geom.value {
        Value::Polygon(v) => pl.push(plane_from_polygon(v)),
        Value::MultiPolygon(v) => v
            .iter()
            .for_each(|poly| pl.push(plane_from_polygon(poly.to_vec()))),
        _ => (),
    }
}

fn plane_from_feature(f: Feature, pl: &mut PlaneList) {
    match f.geometry {
        Some(geom) => plane_from_geometry(geom, pl),
        None => (),
    }
}

fn get_planes(gj: &GeoJson) -> PlaneList {
    match gj {
        GeoJson::FeatureCollection(fc) => {
            let mut pl = PlaneList::new();
            fc.features
                .iter()
                .for_each(|f| plane_from_feature(f.clone(), &mut pl));
            pl
        }
        _ => vec![],
    }
}

fn get_properties(gj: &GeoJson) -> PropList {
    match gj {
        GeoJson::FeatureCollection(fc) => {
            fc.features.iter().map(|f| f.clone().properties).collect()
        }
        _ => vec![],
    }
}

struct BBox {
    minx: f64,
    miny: f64,
    minz: f64,
    maxx: f64,
    maxy: f64,
    maxz: f64,
}

impl BBox {
    fn from_planes(pl: &PlaneList) -> BBox {
        let mut minx = OrderedFloat(std::f64::MAX);
        let mut miny = OrderedFloat(std::f64::MAX);
        let mut minz = OrderedFloat(std::f64::MAX);
        let mut maxx = OrderedFloat(std::f64::MIN);
        let mut maxy = OrderedFloat(std::f64::MIN);
        let mut maxz = OrderedFloat(std::f64::MIN);

        pl.iter().for_each(|plane| {
            plane.iter().for_each(|pt| {
                minx = cmp::min(minx, OrderedFloat(pt.x));
                miny = cmp::min(miny, OrderedFloat(pt.y));
                minz = cmp::min(minz, OrderedFloat(pt.z));
                maxx = cmp::max(maxx, OrderedFloat(pt.x));
                maxy = cmp::max(maxy, OrderedFloat(pt.y));
                maxz = cmp::max(maxz, OrderedFloat(pt.z));
            })
        });
        BBox {
            minx: minx.into_inner(),
            miny: miny.into_inner(),
            minz: minz.into_inner(),
            maxx: maxx.into_inner(),
            maxy: maxy.into_inner(),
            maxz: maxz.into_inner(),
        }
    }

    fn center(&self) -> Point {
        Point::new(
            self.minx + ((self.maxx - self.minx) / 2.0),
            self.miny + ((self.maxy - self.miny) / 2.0),
            self.minz + ((self.maxz - self.minz) / 2.0),
        )
    }

    fn width(&self) -> f64 {
        self.maxx - self.minx
    }

    fn height(&self) -> f64 {
        self.maxz
    }

    fn top_left_near(&self) -> Point {
        Point::new(self.minx, self.miny, self.maxz)
    }
}

// fn bbox(p: Plane) -> BBox {
// let mut minx = f64::INFINITY;
// let mut miny = f64::INFINITY;
// let mut maxx = f64::NEG_INFINITY;
// let mut maxy = f64::NEG_INFINITY;

// p.iter().for_each(|pt| {
//     minx
// })

// }

#[derive(Copy, Clone, Debug)]
struct Camera {
    eye: Point,
    target: Point,
}

fn move_cam(mt: na::Matrix4<f64>, cam: &Camera) -> Camera {
    Camera {
        eye: mt.transform_point(&cam.eye),
        target: mt.transform_point(&cam.target),
    }
}

fn move_eye(mt: na::Matrix4<f64>, cam: &Camera) -> Camera {
    Camera {
        eye: mt.transform_point(&cam.eye),
        target: cam.target.clone(),
    }
}

fn move_target(mt: na::Matrix4<f64>, cam: &Camera) -> Camera {
    Camera {
        eye: cam.eye.clone(),
        target: mt.transform_point(&cam.target),
    }
}

fn rotate_eye(axis: &na::Unit<na::Vector3<f64>>, angle: f64, cam: &Camera) -> Camera {
    let tr = na::Translation3::new(cam.target.x, cam.target.y, cam.target.z).to_homogeneous();
    let itr = na::Translation3::new(-cam.target.x, -cam.target.y, -cam.target.z).to_homogeneous();
    let mat = na::Matrix4::from_axis_angle(axis, angle);
    move_eye(tr * mat * itr, cam)
}

// fn rotate_target(vec: na::Vector3<f64>, angle: f64, cam: &Camera) -> Camera {
//     let axis = na::Unit::new_normalize(vec);
//     let tr = na::Translation3::new(cam.eye.x, cam.eye.y, cam.eye.z).to_homogeneous();
//     let itr = na::Translation3::new(-cam.eye.x, -cam.eye.y, -cam.eye.z).to_homogeneous();
//     let mat = na::Matrix4::from_axis_angle(&axis, angle);
//     move_target(tr * mat * itr, cam)
// }

fn match_mod<F0, F1>(kmod: sdl2::keyboard::Mod, naked: F0, controled: F1) -> Option<Camera>
where
    F0: FnOnce() -> Camera,
    F1: FnOnce() -> Camera,
    // F2: FnOnce() -> Camera,
{
    let ctrl_mod: sdl2::keyboard::Mod = sdl2::keyboard::LCTRLMOD; //& sdl2::keyboard::RCTRLMOD;
                                                                  // let shift_mod: sdl2::keyboard::Mod = sdl2::keyboard::LSHIFTMOD; //& sdl2::keyboard::RSHIFTMOD;
    if kmod.intersects(ctrl_mod) {
        println!("CTRL");
        Some(controled())
    // } else if kmod.intersects(shift_mod) {
    //     println!("SHIFT");
    //     shifted()
    } else {
        println!("NAKED");
        Some(naked())
    }
}

static CAMP_STEP: f64 = 1.2;
static CAM_STEP_ROT: f64 = 0.0174533;

fn cross(a: &Point, b: &Point) -> na::Vector3<f64> {
    let cx = a.y * b.z - a.z * b.y;
    let cy = a.z * b.x - a.x * b.z;
    let cz = a.x * b.y - a.y * b.x;

    na::Vector3::new(cx, cy, cz)
}

fn cross_norm(a: &Point, b: &Point) -> na::Unit<na::Vector3<f64>> {
    na::Unit::new_normalize(cross(a, b))
}
fn get_horizontal_axis(c: &Camera) -> na::Unit<na::Vector3<f64>> {
    let pt0 = Point::new(c.eye.x - c.target.x, c.eye.y - c.target.y, c.eye.z);
    let pt1 = Point::new(c.eye.x - c.target.x, c.eye.y - c.target.y, c.target.z);

    cross_norm(&pt0, &pt1)
}

fn deg_to_rad(a: f64) -> f64 {
    a * std::f64::consts::PI / 180.0
}

fn vertical_axis() -> na::Unit<na::Vector3<f64>> {
    na::Unit::new_normalize(na::Vector3::new(0.0, 0.0, 1.0))
}

fn side_mov(c: &Camera, step: f64) -> na::Matrix4<f64> {
    let axis = get_horizontal_axis(c);
    let m = axis.unwrap() * step;
    let tr = na::Translation3::from(m);

    tr.to_homogeneous()
}

fn axis_mov(cam: &Camera, step: f64) -> na::Matrix4<f64> {
    let axis = na::Unit::new_normalize(cam.target - cam.eye);
    let m = axis.unwrap() * step;
    let tr = na::Translation3::from(m);
    // println!(
    //     "axis_mov \n{}\n{}\n{}",
    //     cam.eye,
    //     tr.to_homogeneous(),
    //     tr.to_homogeneous().transform_point(&cam.eye)
    // );
    tr.to_homogeneous()
}

fn app<A, T, F: FnOnce(A) -> T>(f: F, a: A) -> T {
    f(a)
}

fn handle_key_event(
    key: Option<sdl2::keyboard::Keycode>,
    kmod: sdl2::keyboard::Mod,
    cam: &Camera,
    initial_camera: &Camera,
    capture: &mut Capture,
) -> Option<Camera> {
    key.and_then(|code| {
        match code {
            // naked  => camera
            // contol => eye
            // shift  => target
            sdl2::keyboard::Keycode::R => Some(*initial_camera),
            sdl2::keyboard::Keycode::C => {
                capture.toggle();
                None
            }
            sdl2::keyboard::Keycode::S => {
                capture.save("capture.cardboard");
                None
            }
            sdl2::keyboard::Keycode::Left => match_mod(
                kmod,
                || move_cam(side_mov(&cam, -CAMP_STEP), cam),
                || rotate_eye(&vertical_axis(), -CAM_STEP_ROT, cam),
            ),
            sdl2::keyboard::Keycode::Right => match_mod(
                kmod,
                || move_cam(side_mov(&cam, CAMP_STEP), cam),
                || rotate_eye(&vertical_axis(), CAM_STEP_ROT, cam),
            ),
            sdl2::keyboard::Keycode::Up => match_mod(
                kmod,
                || move_eye(axis_mov(&cam, -CAMP_STEP), cam),
                || rotate_eye(&get_horizontal_axis(&cam), CAM_STEP_ROT, cam),
            ),
            sdl2::keyboard::Keycode::Down => match_mod(
                kmod,
                || move_eye(axis_mov(&cam, CAMP_STEP), cam),
                || rotate_eye(&get_horizontal_axis(&cam), -CAM_STEP_ROT, cam),
            ),
            _ => None,
        }
    })
}

fn handle_motion_event(xrel: i32, yrel: i32, cam: &Camera) -> Option<Camera> {
    let ox = f64::from(xrel) / 6.0;
    let oy = f64::from(yrel) / 6.0;

    let horizontal_axis = get_horizontal_axis(cam);
    let vertical_axis = na::Unit::new_normalize(na::Vector3::new(0.0, 0.0, 1.0));
    let tr = na::Translation3::new(cam.eye.x, cam.eye.y, cam.eye.z).to_homogeneous();
    let itr = na::Translation3::new(-cam.eye.x, -cam.eye.y, -cam.eye.z).to_homogeneous();
    let hmat = na::Matrix4::from_axis_angle(&horizontal_axis, deg_to_rad(oy));
    let vmat = na::Matrix4::from_axis_angle(&vertical_axis, deg_to_rad(ox));
    let op = match (xrel, yrel) {
        (0, 0) => na::Matrix4::identity(),
        (0, _) => tr * vmat * itr,
        (_, 0) => tr * hmat * itr,
        (_, _) => tr * vmat * hmat * itr,
    };
    // println!("{:?} {:?}", horizontal_axis, vertical_axis);

    // println!("handle_motion_event {} {}", ox, oy);
    // println!("before {}", cam.target);
    // println!("after {}", op.transform_point(&cam.target));

    Some(move_target(op, cam))
}

fn handle_wheel_event(y: i32, cam: &Camera) -> Option<Camera> {
    let oy = f64::from(y);
    println!("handle_wheel_event {} {}", oy, CAMP_STEP * oy);
    Some(move_target(axis_mov(&cam, CAMP_STEP * oy), cam))
}

fn sort_planes(p: Point, pl: &PlaneList) -> Vec<usize> {
    let mut indices: Vec<usize> = Vec::with_capacity(pl.len());
    // let mut distances: Vec<OrderedFloat<f64>> = Vec::with_capacity(pl.len());

    let start = PreciseTime::now();
    for i in 0..pl.len() {
        indices.push(i);
        // distances.push(pl[i].iter().fold(OrderedFloat(0.0), |acc, v| {
        //     cmp::max(OrderedFloat(distance_squared(&p, v)), acc)
        // }));
    }

    let distances: Vec<OrderedFloat<f64>> = indices
        .par_iter()
        .map(|i| {
            pl[i.to_owned()].iter().fold(OrderedFloat(0.0), |acc, v| {
                cmp::max(OrderedFloat(distance_squared(&p, v)), acc)
            })
        }).collect();

    let end = PreciseTime::now();
    println!("Distances in {}", start.to(end));

    indices.par_sort_unstable_by(|a, b| {
        let da = &distances[a.to_owned()];
        let db = &distances[b.to_owned()];

        if da < db {
            cmp::Ordering::Greater
        } else if da > db {
            cmp::Ordering::Less
        } else {
            cmp::Ordering::Equal
        }
    });
    // println!("Indices --");
    // indices.iter().for_each(|i| println!("> {}", i));

    indices
}

// type M3 = na::Matrix3<f64>;

fn make_cross(pt: Point, cin: f64, cout: f64) -> Plane {
    let x = pt.x;
    let y = pt.y;
    let z = pt.z;
    vec![
        Point::new(x - cin, y + cin, z),
        Point::new(x - cin, y + cout, z),
        Point::new(x + cin, y + cout, z),
        Point::new(x + cin, y + cin, z),
        Point::new(x + cout, y + cin, z),
        Point::new(x + cout, y - cin, z),
        Point::new(x + cin, y - cin, z),
        Point::new(x + cin, y - cout, z),
        Point::new(x - cin, y - cout, z),
        Point::new(x - cin, y - cin, z),
        Point::new(x - cout, y - cin, z),
        Point::new(x - cout, y + cin, z),
    ]
}

fn transform2d(
    aligned_point3d: &Point,
    corrective: &na::Matrix3<f64>,
    scale: f64,
    tr: &na::Matrix3<f64>,
) -> na::Point2<f64> {
    let aligned_point = na::Point2::new(aligned_point3d.x, aligned_point3d.y);
    let rotated = corrective.transform_point(&aligned_point);
    let scaled = rotated * scale;
    tr.transform_point(&scaled)
}

fn draw_planes(
    pl: &PlaneList,
    cam: &Camera,
    sdl_texture: &mut sdl2::render::Texture,
    style_list: &StyleList,
) {
    let sdl_query = sdl_texture.query();
    let rect = sdl2::rect::Rect::new(0, 0, sdl_query.width, sdl_query.height);
    sdl_texture
        .with_lock(Some(rect), |sdl_data, stride| {
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

            // 149144.0	17115cout
            let scale = na::distance(&cam.eye, &cam.target).abs();
            let iscale = f64::from(sdl_query.width) / scale;
            let target_ref = Point::new(cam.target.x, cam.target.y, cam.target.z + 10.0);

            let indices = sort_planes(cam.eye, pl);

            let view = Isometry3::look_at_rh(&cam.eye, &cam.target, &Vector3::z()).to_homogeneous();

            println!(
                "Orthographic3::new({}, {}, {}, {}, {}, {})",
                -iscale, iscale, -iscale, iscale, 0.0, scale
            );
            let proj_o =
                na::geometry::Orthographic3::new(-iscale, iscale, -iscale, iscale, 1.0, scale)
                    .unwrap();
            let vp = proj_o * view;

            let projected_target_ref = vp.transform_point(&target_ref);
            let target_ref_angle = na::angle(
                &na::Vector2::new(0.0, -1.0),
                &na::Vector2::new(projected_target_ref.x, projected_target_ref.y),
            );
            let corrective = if projected_target_ref.x < 0.0 {
                na::Matrix3::new_rotation(target_ref_angle)
            } else {
                na::Matrix3::new_rotation(-target_ref_angle)
            };

            let translation = f64::from(sdl_query.width) / 2.0;
            let tr = na::Translation2::new(translation, translation).to_homogeneous();

            let clip_z = view.transform_point(&cam.eye).z;

            let mut started = false;
            context.new_path();
            make_cross(cam.target, 1.0, 2.0)
                .iter()
                .map(|pt| vp.transform_point(pt))
                .map(|pt| transform2d(&pt, &corrective, scale, &tr))
                .for_each(|pt| {
                    if started {
                        context.line_to(pt.x, pt.y);
                    } else {
                        started = true;
                        context.move_to(pt.x, pt.y);
                    }
                });
            context.close_path();
            context.set_line_width(0.2);
            context.set_source_rgb(1.0, 0.0, 0.0);
            // context.fill_preserve();
            context.stroke();

            indices.iter().for_each(|index| {
                // println!("drawing {}", index);
                let mut started = false;
                let plane = &pl[index.to_owned()];
                context.new_path();

                // let vs: Plane = plane.iter().map(|pt| view.transform_point(pt)).collect();
                let als: Plane = plane.iter().map(|pt| vp.transform_point(pt)).collect();
                let is_in_front = plane.iter().any(|p| {
                    // p.x > -iscale
                    //     && p.x < iscale
                    //     && p.y > -iscale
                    //     && p.y < iscale
                    //     && p.z > 0.0
                    //     && p.z < scale
                    view.transform_point(p).z < clip_z
                });

                if is_in_front {
                    als.iter().for_each(|aligned_point3d| {
                        let translated = transform2d(aligned_point3d, &corrective, scale, &tr);
                        if started {
                            context.line_to(translated.x, translated.y);
                        // println!("  LINE {} {} ({})", pp[0], pp[1], pp[2])
                        } else {
                            started = true;
                            context.move_to(translated.x, translated.y);
                            // println!("MOVE {} {}", tp[0], tp[1])
                        }
                    });
                    context.close_path();
                    style_list.get_for(index).map(|s| {
                        s.fillColor.map(|color| {
                            context.set_source_rgba(
                                color.red,
                                color.green,
                                color.blue,
                                color.alpha,
                            );
                            context.fill_preserve();
                        });

                        s.strokeColor.map(|color| {
                            context.set_line_width(s.strokeWidth);
                            context.set_source_rgba(
                                color.red,
                                color.green,
                                color.blue,
                                color.alpha,
                            );
                            context.stroke();
                        });
                    });
                }
            });

            started = false;
            context.new_path();
            make_cross(cam.eye, 0.5, 1.0)
                .iter()
                .map(|pt| vp.transform_point(pt))
                .map(|pt| transform2d(&pt, &corrective, scale, &tr))
                .for_each(|pt| {
                    if started {
                        context.line_to(pt.x, pt.y);
                    } else {
                        started = true;
                        context.move_to(pt.x, pt.y);
                    }
                });
            context.close_path();
            context.set_line_width(0.2);
            context.set_source_rgb(0.0, 0.0, 1.0);
            context.stroke();

            // let eye_t = transform2d(&vp.transform_point(&cam.eye), &corrective, scale, &tr);
            // let tar_t = transform2d(&vp.transform_point(&cam.target), &corrective, scale, &tr);
            // context.new_path();
            // context.move_to(tar_t.x, tar_t.y);
            // context.line_to(eye_t.x, eye_t.y);
            // context.set_line_width(2.0);
            // context.set_source_rgb(1.0, 1.0, 0.0);
            // context.stroke();

            // println!(
            //     "--\n{}\n{}\n--",
            //     view.transform_point(&cam.eye),
            //     view.transform_point(&cam.target)
            // );
        }).unwrap();
}

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
            Camera {
                eye: Point::new(cam_x, cam_y, cam_z),
                target: Point::new(target_x, target_y, target_z),
            },
        ))
    }
}

struct Capture {
    on: bool,
    frames: Vec<Frame>,
}

impl Capture {
    fn new() -> Capture {
        Capture {
            on: false,
            frames: Vec::new(),
        }
    }

    fn toggle(&mut self) {
        if self.on {
            self.on = false;
        } else {
            self.on = true;
            self.frames = Vec::new();
        }
    }

    fn map(&mut self, timetamp: u32, camera: Camera) {
        if self.on {
            self.frames.push(Frame::new(timetamp, camera));
        }
    }

    fn save(&self, file_path: &str) -> std::io::Result<()> {
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

    fn from_records(file_path: &str) -> std::io::Result<Capture> {
        let records = std::fs::read_to_string(file_path)?;
        let mut frames: Vec<Frame> = Vec::new();

        records.lines().for_each(|r| {
            Frame::from_record(r).map(|f| frames.push(f));
        });

        Ok(Capture { on: false, frames })
    }
}

fn update_cam(
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

fn replay(capture: &Capture) {
    capture
        .frames
        .iter()
        .for_each(|f| println!("R> {}", f.timestamp));
}

fn main() {
    let sdl = sdl2::init().unwrap();
    // sdl.mouse().set_relative_mouse_mode(true);
    let video_subsystem = sdl.video().unwrap();
    let window = video_subsystem
        .window("Cardoard", 800, 800)
        .build()
        .unwrap();

    let mut event_pump = sdl.event_pump().unwrap();

    let args: Vec<String> = env::args().collect();
    let data_fn = &args[1];
    let style_fn = &args[2];

    if args.len() == 4 {
        let cap_file = &args[3];
        Capture::from_records(cap_file).map(|c| replay(&c));
    }

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
    let mut camera = initial_camera;

    let mut canvas = sdl2::render::CanvasBuilder::new(window).build().unwrap();
    canvas.set_draw_color(sdl2::pixels::Color::RGB(100, 100, 100));
    canvas.clear();
    let texture_creator = canvas.texture_creator();
    let mut sdl_texture: sdl2::render::Texture = texture_creator
        .create_texture(
            Some(sdl2::pixels::PixelFormatEnum::ARGB8888),
            sdl2::render::TextureAccess::Streaming,
            800,
            800,
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
                    update_cam(
                        handle_key_event(keycode, keymod, &camera, &initial_camera, &mut capture),
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
                        update_cam(
                            handle_motion_event(xrel, yrel, &camera),
                            timestamp,
                            &mut dirty,
                            &mut camera,
                            &mut capture,
                        );
                    }
                }
                sdl2::event::Event::MouseWheel { y, timestamp, .. } => {
                    update_cam(
                        handle_wheel_event(y, &camera),
                        timestamp,
                        &mut dirty,
                        &mut camera,
                        &mut capture,
                    );
                }

                _ => {}
            }
        }

        // render window contents here
        if dirty {
            canvas.clear();
            draw_planes(&pl, &camera, &mut sdl_texture, &style);
            match canvas.copy(&sdl_texture, None, None) {
                Ok(_) => {
                    canvas.present();
                    dirty = false;
                }
                _ => dirty = true,
            };
        }
        // let result = canvas.with_texture_canvas(&mut texture, |texture_canvas| {
        //     texture_canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        //     texture_canvas.clear();
        //     texture_canvas.set_draw_color(Color::RGBA(255, 0, 0, 255));
        //     texture_canvas.fill_rect(Rect::new(50, 50, 50, 50)).unwrap();
        // });
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use cross;
    use nalgebra as na;
    use Point;
    #[test]
    fn cross_product() {
        let a = Point::new(1.0, 2.0, 3.0);
        let b = Point::new(1.0, 5.0, 7.0);
        let c = cross(&a, &b);
        // println!("{:?}", c);
        assert_eq!(c, na::Vector3::new(-1.0, -4.0, 3.0))
    }
}
