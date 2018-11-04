extern crate cairo;
extern crate cairo_sys;
extern crate geojson;
extern crate libc;
extern crate nalgebra;
extern crate ordered_float;
extern crate sdl2;
extern crate time;

mod data;
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
use std::cmp;
use std::env;
use std::fs::File;
use surface_data::create_for_data_unsafe;
use time::PreciseTime;

pub type Point = Point3<f64>;
pub type Plane = Vec<Point>;
pub type PlaneList = Vec<Plane>;

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

fn extract_features(gj: GeoJson) -> PlaneList {
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
        let mut minx = OrderedFloat(std::f64::INFINITY);
        let mut miny = OrderedFloat(std::f64::INFINITY);
        let mut minz = OrderedFloat(std::f64::INFINITY);
        let mut maxx = OrderedFloat(std::f64::NEG_INFINITY);
        let mut maxy = OrderedFloat(std::f64::NEG_INFINITY);
        let mut maxz = OrderedFloat(std::f64::NEG_INFINITY);

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

fn match_mod<F0, F1>(kmod: sdl2::keyboard::Mod, left: F0, right: F1) -> Camera
where
    F0: FnOnce() -> Camera,
    F1: FnOnce() -> Camera,
{
    let ctrl_mod: sdl2::keyboard::Mod = sdl2::keyboard::LCTRLMOD; //& sdl2::keyboard::RCTRLMOD;
    if kmod.intersects(ctrl_mod) {
        left()
    } else {
        right()
    }
}

fn rotate_cam(vec: na::Vector3<f64>, angle: f64, cam: &Camera) -> Camera {
    let axis = na::Unit::new_normalize(vec);
    let tr = na::Translation3::new(cam.target.x, cam.target.y, cam.target.z).to_homogeneous();
    let itr = na::Translation3::new(-cam.target.x, -cam.target.y, -cam.target.z).to_homogeneous();
    let mat = na::Matrix4::from_axis_angle(&axis, angle);
    move_eye(tr * mat * itr, cam)
}

static CAMP_STEP: f64 = 1.0;
static CAM_STEP_ROT: f64 = 0.0174533;

fn handle_kevent(
    key: Option<sdl2::keyboard::Keycode>,
    kmod: sdl2::keyboard::Mod,
    cam: &Camera,
) -> Option<Camera> {
    key.map(|code| match code {
        sdl2::keyboard::Keycode::Left => match_mod(
            kmod,
            || rotate_cam(na::Vector3::new(0.0, 0.0, 1.0), -CAM_STEP_ROT, cam),
            || {
                move_cam(
                    na::Translation3::new(-CAMP_STEP, 0.0, 0.0).to_homogeneous(),
                    cam,
                )
            },
        ),
        sdl2::keyboard::Keycode::Right => match_mod(
            kmod,
            || rotate_cam(na::Vector3::new(0.0, 0.0, 1.0), CAM_STEP_ROT, cam),
            || {
                move_cam(
                    na::Translation3::new(CAMP_STEP, 0.0, 0.0).to_homogeneous(),
                    cam,
                )
            },
        ),
        sdl2::keyboard::Keycode::Up => match_mod(
            kmod,
            || {
                let e = na::Vector3::new(cam.eye.x, cam.eye.y, cam.eye.z);
                let c = na::Vector3::new(cam.eye.x, cam.eye.y, 0.0).cross(&e);

                rotate_cam(c, CAM_STEP_ROT, cam)
            },
            || {
                move_target(
                    na::Translation3::new(0.0, 0.0, CAMP_STEP).to_homogeneous(),
                    cam,
                )
            },
        ),
        sdl2::keyboard::Keycode::Down => match_mod(
            kmod,
            || {
                let e = na::Vector3::new(cam.eye.x, cam.eye.y, cam.eye.z);
                let c = na::Vector3::new(cam.eye.x, cam.eye.y, 0.0).cross(&e);

                rotate_cam(c, -CAM_STEP_ROT, cam)
            },
            || {
                move_target(
                    na::Translation3::new(0.0, 0.0, -CAMP_STEP).to_homogeneous(),
                    cam,
                )
            },
        ),
        _ => cam.clone(),
    })
}

fn sort_planes(p: Point, pl: &PlaneList) -> Vec<usize> {
    let mut indices: Vec<usize> = Vec::with_capacity(pl.len());
    let mut distances: Vec<OrderedFloat<f64>> = Vec::with_capacity(pl.len());

    let start = PreciseTime::now();
    for i in 0..pl.len() {
        indices.push(i);
        distances.push(pl[i].iter().fold(OrderedFloat(0.0), |acc, v| {
            cmp::max(OrderedFloat(distance_squared(&p, v)), acc)
        }));
    }
    let end = PreciseTime::now();
    println!("Distances in {}", start.to(end));

    indices.sort_unstable_by(|a, b| {
        let da = &distances[a.to_owned()];
        let db = &distances[b.to_owned()];

        if da < db {
            cmp::Ordering::Less
        } else if da > db {
            cmp::Ordering::Greater
        } else {
            cmp::Ordering::Equal
        }
    });
    // println!("Indices --");
    // indices.iter().for_each(|i| println!("> {}", i));

    indices
}

// type M3 = na::Matrix3<f64>;

fn draw_planes(pl: &PlaneList, cam: &Camera, sdl_texture: &mut sdl2::render::Texture) {
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

            // let surface = ImageSurface::create(Format::ARgb32, screen_width, screen_height)
            //     .expect("Couldn’t create a surface!");
            let context = Context::new(&surface);

            context.set_source_rgb(1.0, 1.0, 1.0);
            context.paint();

            // 149144.0	171151.0
            let scale = na::distance(&cam.eye, &cam.target);
            let iscale = f64::from(sdl_query.width) / scale;
            let target_ref = Point::new(cam.target.x, cam.target.y, cam.target.z + 10.0);

            // let center = Point::new(149144.0, 171151.0, scale);
            // let bottom_left_near = Point::new(center.x - scale, center.y - scale, 0.0);
            // let top_right_far = Point::new(center.x + scale, center.y + scale, 2.0 * scale);

            // let eye = top_right_far; //Point3::new(149140.0 - 20.0, 171151.0, 1.0);
            // let target = center; //Point3::new(149144.0, 171151.0 - 100.0, 40.0);

            let indices = sort_planes(cam.eye, pl);

            let view = Isometry3::look_at_lh(&cam.eye, &cam.target, &Vector3::z()).to_homogeneous();

            // let view_bl = view.transform_point(&bottom_left_near);
            // let view_tr = view.transform_point(&top_right_far);
            // let left = view_bl.x;
            // let bottom = view_bl.y;
            // let right = view_tr.x;
            // let top = view_tr.y;
            // let near = view_bl.z;
            // let far = view_tr.z;

            // println!("left   {}", left);
            // println!("bottom {}", bottom);
            // println!("right  {}", right);
            // println!("top    {}", top);
            // println!("near   {}", near);
            // println!("far    {}", far);
            // println!("--");

            // let proj_o = if near < far {
            //     na::geometry::Orthographic3::new(-iscale, iscale, -iscale, iscale, near, far)
            //         .unwrap()
            // } else {
            //     na::geometry::Orthographic3::new(-iscale, iscale, -iscale, iscale, far, near)
            //         .unwrap()
            // };
            // let _proj_p = Perspective3::new(1.0, 3.14 / 4.0, 10.0, 100.0).unwrap();
            let proj_o =
                na::geometry::Orthographic3::new(-iscale, iscale, -iscale, iscale, 0.0, scale)
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

            // let projected_bl = vp.transform_point(&bottom_left_near);
            // let projected_tr = vp.transform_point(&top_right_far);
            // let _projected_width = projected_tr.x - projected_bl.x;
            // let ratio = f64::from(screen_width) / projected_width;
            let translation = f64::from(sdl_query.width) / 2.0;
            // println!("projected_bl {}", projected_bl);
            // println!("projected_tr {}", projected_tr);
            // println!("translation  {}", translation);
            // println!("--");

            let tr = na::Translation2::new(translation, translation).to_homogeneous();

            indices.iter().for_each(|index| {
                // println!("drawing {}", index);
                let mut started = false;
                let plane = &pl[index.to_owned()];
                plane.iter().for_each(|pt| {
                    let aligned_point3d = vp.transform_point(pt);
                    let aligned_point = na::Point2::new(aligned_point3d.x, aligned_point3d.y);
                    let rotated = corrective.transform_point(&aligned_point);
                    let scaled = rotated * scale;
                    let translated = tr.transform_point(&scaled);
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
                context.set_source_rgb(0.9, 0.9, 0.9);
                context.fill_preserve();

                context.set_line_width(0.8);
                context.set_source_rgb(0.0, 0.0, 0.0);
                context.stroke();
            });
            // let mut pp = vp.transform_point(&bottom_left_near);
            // println!("bottom_left           {}", bottom_left_near);
            // println!(
            //     "bottom_left * view    {}",
            //     view.transform_point(&bottom_left_near)
            // );
            // println!(
            //     "bottom_left * proj_o  {}",
            //     proj_o.transform_point(&bottom_left_near)
            // );
            // println!("bottom_left * vp      {}", pp);
            // println!("view_bottom_left * tr      {}", tr.transform_point(&pp));
            // println!("--");

            // pp = vp.transform_point(&top_right_far);
            // println!("top_right           {}", top_right_far);
            // println!(
            //     "top_right * view    {}",
            //     view.transform_point(&top_right_far)
            // );
            // println!(
            //     "top_right * proj_o  {}",
            //     proj_o.transform_point(&top_right_far)
            // );
            // println!("top_right * vp      {}", pp);
            // println!("view_top_right * tr      {}", tr.transform_point(&pp));

            // let mut file = File::create("output.png").expect("Couldn’t create file.");
            // surface
            //     .write_to_png(&mut file)
            //     .expect("Couldn’t write to png");
        }).unwrap();
}

fn main() {
    let sdl = sdl2::init().unwrap();
    let video_subsystem = sdl.video().unwrap();
    let window = video_subsystem
        .window("Cardoard", 800, 800)
        .build()
        .unwrap();

    let mut event_pump = sdl.event_pump().unwrap();

    let args: Vec<String> = env::args().collect();
    let filename = &args[1];

    let pl = load_geojson(filename).map(|gj| extract_features(gj));

    match pl {
        Err(_) => println!("failed"),
        Ok(pl) => {
            println!("N {}", pl.len());

            let bbox = BBox::from_planes(&pl);
            let mut camera = Camera {
                eye: bbox.top_left_near(),
                target: bbox.center(),
            };
            let mut canvas = sdl2::render::CanvasBuilder::new(window).build().unwrap();
            canvas.set_draw_color(sdl2::pixels::Color::RGB(100, 100, 100));
            canvas.clear();
            let texture_creator = canvas.texture_creator();
            let mut sdl_texture: sdl2::render::Texture = texture_creator
                .create_texture(
                    Some(sdl2::pixels::PixelFormatEnum::ABGR8888),
                    sdl2::render::TextureAccess::Streaming,
                    800,
                    800,
                ).unwrap();

            let mut dirty = true;

            'main: loop {
                for event in event_pump.poll_iter() {
                    match event {
                        sdl2::event::Event::Quit { .. } => break 'main,
                        sdl2::event::Event::KeyDown {
                            keycode, keymod, ..
                        } => {
                            match handle_kevent(keycode, keymod, &camera) {
                                Some(c) => {
                                    camera = c;

                                    dirty = true;
                                }
                                None => (),
                            };
                            println!("> {:?}", camera);
                        }
                        _ => {}
                    }
                }

                // render window contents here
                if dirty {
                    canvas.clear();
                    draw_planes(&pl, &camera, &mut sdl_texture);
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
    }
}
