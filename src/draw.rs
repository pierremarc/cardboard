use camera::Camera;
use geom::transform2d;
use lingua::Plane;
use lingua::PlaneList;
use lingua::Point;
use nalgebra as na;
use nalgebra::distance_squared;
use operation::OpList;
use operation::Operation;
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use std::cmp;
use std::iter::{Flatten, Map};
use style::StyleList;
use time::PreciseTime;

fn sort_planes(p: Point, pl: &PlaneList) -> Vec<usize> {
    let mut indices: Vec<usize> = Vec::with_capacity(pl.len());

    // let start = PreciseTime::now();
    for i in 0..pl.len() {
        indices.push(i);
    }

    let distances: Vec<OrderedFloat<f64>> = indices
        .par_iter()
        .map(|i| {
            pl[i.to_owned()].iter().fold(OrderedFloat(0.0), |acc, v| {
                cmp::max(OrderedFloat(distance_squared(&p, v)), acc)
            })
        }).collect();

    let end = PreciseTime::now();
    // println!("Distances in {}", start.to(end));

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

    indices
}

pub type PlanePainter = Fn(usize) -> Vec<Operation>;
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
    let als: Plane = plane
        .iter()
        .map(|pt| view_projection.transform_point(pt))
        .collect();
    let is_in_front = plane.iter().any(|p| view.transform_point(p).z < clip_z);

    if is_in_front {
        ops.push(Operation::Begin);
        als.iter().for_each(|aligned_point3d| {
            let translated = transform2d(aligned_point3d, &corrective, scale, &tr);
            if started {
                ops.push(Operation::Line(translated.to_owned()));
            } else {
                started = true;
                ops.push(Operation::Move(translated.to_owned()));
            }
            ops.push(Operation::Close);
            ops.push(Operation::Paint(index.to_owned()));
        });
    };
    ops
}

pub fn draw_planes(pl: &PlaneList, cam: &Camera, style_list: &StyleList, width: f64) -> OpList {
    // 149144.0	17115cout
    let scale = na::distance(&cam.eye, &cam.target).abs();
    let iscale = width / scale;
    let target_ref = Point::new(cam.target.x, cam.target.y, cam.target.z + 10.0);

    let indices = sort_planes(cam.eye, pl);

    let view = na::geometry::Isometry3::look_at_rh(&cam.eye, &cam.target, &na::Vector3::z())
        .to_homogeneous();

    // println!(
    //     "Orthographic3::new({}, {}, {}, {}, {}, {})",
    //     -iscale, iscale, -iscale, iscale, 0.0, scale
    // );
    // let proj_o =
    //     na::geometry::Orthographic3::new(-iscale, iscale, -iscale, iscale, 1.0, scale)
    //         .unwrap();
    let proj_o =
        na::geometry::Orthographic3::new(-iscale, iscale, -iscale, iscale, -scale, 1.0).unwrap();
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

    let translation = width / 2.0;
    let tr = na::Translation2::new(translation, translation).to_homogeneous();

    let clip_z = view.transform_point(&cam.eye).z;

    indices
        .iter()
        .map(|index| {
            draw_index(
                index.to_owned(),
                pl,
                &view,
                &vp,
                &corrective,
                clip_z,
                scale,
                &tr,
            )
        }).flatten()
        .collect()
}
