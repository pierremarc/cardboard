use camera::Camera;
use geom::transform2d;
use lingua::PlaneList;
use lingua::Point;
use nalgebra as na;
use nalgebra::distance_squared;
use operation::{OpList, Operation};
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use std::cmp;
use time::PreciseTime;

struct Dist(OrderedFloat<f64>, usize);

// fn sort_planes(p: Point, pl: &PlaneList) -> Vec<usize> {
//     let mut indices: Vec<usize> = Vec::with_capacity(pl.len());

//     let mut start = PreciseTime::now();
//     for i in 0..pl.len() {
//         indices.push(i);
//     }

//     let distances: Vec<Dist> = indices
//         .par_iter()
//         .map(|i| {
//             let plane = &pl[i.to_owned()];
//             let d = plane.points.iter().fold(OrderedFloat(0.0), |acc, v| {
//                 cmp::max(OrderedFloat(distance_squared(&p, v)), acc)
//             });
//             Dist(d, plane.layer_index)
//         }).collect();

//     println!("Distances in {}", start.to(PreciseTime::now()));
//     start = PreciseTime::now();
//     indices.par_sort_unstable_by(|a, b| {
//         let da = &distances[a.to_owned()];
//         let db = &distances[b.to_owned()];

//         if da.0 < db.0 {
//             cmp::Ordering::Greater
//         } else if da.0 > db.0 {
//             cmp::Ordering::Less
//         } else {
//             if da.1 < db.1 {
//                 cmp::Ordering::Greater
//             } else if da.1 > db.1 {
//                 cmp::Ordering::Less
//             } else {
//                 cmp::Ordering::Equal
//             }
//         }
//     });

//     println!("Sort in {}", start.to(PreciseTime::now()));
//     indices
// }

// pub type PlanePainter = Fn(usize) -> Vec<Operation>;
// pub type FlattenedOps = Flatten<Map<std::slice::Iter<'_, usize>>>;

fn draw_index(
    index: usize,
    pl: &PlaneList,
    view: &na::Matrix4<f64>,
    view_projection: &na::Matrix4<f64>,
    corrective: &na::Matrix3<f64>,
    clip_z: f64,
    scale: f64,
    tr: &na::Matrix3<f64>,
) -> Vec<Operation> {
    let mut ops: Vec<Operation> = Vec::new();
    let mut started = false;
    let plane = &pl[index];
    let als = plane
        .points
        .iter()
        .map(|pt| view_projection.transform_point(pt));
    let is_in_front = plane.points.iter().any(|p| view.transform_point(p).z < 0.0);

    if is_in_front {
        ops.push(Operation::Begin);
        als.for_each(|aligned_point3d| {
            let translated = transform2d(&aligned_point3d, &corrective, scale, &tr);
            if started {
                ops.push(Operation::Line(translated.to_owned()));
            } else {
                started = true;
                ops.push(Operation::Move(translated.to_owned()));
            }
        });
        ops.push(Operation::Close);
        ops.push(Operation::Paint(plane.layer_index, index.to_owned()));
    };
    ops
}

pub struct DrawConfig {
    indices: Vec<usize>,
    view: na::Matrix4<f64>,
    view_projection: na::Matrix4<f64>,
    corrective: na::Matrix3<f64>,
    clip_z: f64,
    scale: f64,
    tr: na::Matrix3<f64>,
}

pub trait Drawable {
    fn sorted_indices(&self, p: Point) -> Vec<usize>;

    fn draw<F>(&self, config: &DrawConfig, f: F)
    where
        F: FnMut(&Operation);
}

impl Drawable for PlaneList {
    fn sorted_indices(&self, p: Point) -> Vec<usize> {
        let mut indices: Vec<usize> = Vec::with_capacity(self.len());

        let mut start = PreciseTime::now();
        for i in 0..self.len() {
            indices.push(i);
        }

        let distances: Vec<Dist> = indices
            .par_iter()
            .map(|i| {
                let plane = &self[i.to_owned()];
                let d = plane.points.iter().fold(OrderedFloat(0.0), |acc, v| {
                    cmp::max(OrderedFloat(distance_squared(&p, v)), acc)
                });
                Dist(d, plane.layer_index)
            }).collect();

        println!("Distances in {}", start.to(PreciseTime::now()));
        start = PreciseTime::now();
        indices.par_sort_unstable_by(|a, b| {
            let da = &distances[a.to_owned()];
            let db = &distances[b.to_owned()];

            if da.0 < db.0 {
                cmp::Ordering::Greater
            } else if da.0 > db.0 {
                cmp::Ordering::Less
            } else {
                if da.1 < db.1 {
                    cmp::Ordering::Greater
                } else if da.1 > db.1 {
                    cmp::Ordering::Less
                } else {
                    cmp::Ordering::Equal
                }
            }
        });

        println!("Sort in {}", start.to(PreciseTime::now()));
        indices
    }

    fn draw<F>(&self, config: &DrawConfig, f: F)
    where
        F: FnMut(&Operation),
    {
        let ops: OpList = config
            .indices
            .par_iter()
            .map(|index| {
                draw_index(
                    index.to_owned(),
                    self,
                    &config.view,
                    &config.view_projection,
                    &config.corrective,
                    config.clip_z,
                    config.scale,
                    &config.tr,
                )
            }).flatten()
            .collect();

        ops.iter().for_each(f);
    }
}

pub fn get_draw_config(pl: &PlaneList, cam: &Camera, width: f64) -> DrawConfig {
    let dist = na::distance(&cam.eye, &cam.target).abs();
    let scale = dist / 2.0;

    let target_ref = Point::new(cam.target.x, cam.target.y, cam.target.z + 10.0);

    let view = na::geometry::Isometry3::look_at_rh(&cam.eye, &cam.target, &na::Vector3::z())
        .to_homogeneous();

    let proj_o =
        na::geometry::Orthographic3::new(-scale, scale, -scale, scale, -2.0 * scale, 0.0).unwrap();

    println!(
        "Orthographic3::new({}, {}, {}, {}, {}, {})",
        -scale,
        scale,
        -scale,
        scale,
        -2.0 * scale,
        0.0
    );

    let view_projection = proj_o * view;

    let projected_target_ref = view_projection.transform_point(&target_ref);
    let target_ref_angle = na::angle(
        &na::Vector2::new(0.0, -1.0),
        &na::Vector2::new(projected_target_ref.x, projected_target_ref.y),
    );
    let corrective = if projected_target_ref.x < 0.0 {
        na::Matrix3::new_rotation(target_ref_angle)
    } else {
        na::Matrix3::new_rotation(-target_ref_angle)
    };

    let translation = width / 2.0;
    let tr = na::Translation2::new(translation, translation).to_homogeneous();

    let clip_z = view.transform_point(&cam.eye).z;

    DrawConfig {
        indices: pl.sorted_indices(cam.eye),
        view,
        view_projection,
        corrective,
        clip_z,
        scale: translation,
        tr,
    }
}
