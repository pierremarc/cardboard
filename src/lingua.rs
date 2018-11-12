use geojson::{Feature, GeoJson, Geometry, PolygonType, Value};
use nalgebra::geometry::{Point2, Point3};
use serde::de::{self, Deserialize, Deserializer, SeqAccess, Visitor};
use serde_json;
use std::iter::Flatten;
use std::slice::Iter;

pub type Properties = std::option::Option<serde_json::Map<std::string::String, serde_json::Value>>;

pub type Point = Point3<f64>;
pub type Point2D = Point2<f64>;

pub struct PlaneT {
    pub layer_index: usize,
    pub style_index: usize,
    pub points: Vec<Point>,
}

pub enum Plane {
    None,
    One(PlaneT),
    Multi(Vec<PlaneT>),
}

impl Plane {
    pub fn iter(&self) -> Iter<PlaneT> {
        match *self {
            Plane::None => Vec::new().iter(),
            Plane::One(plane) => vec![plane].iter(),
            Plane::Multi(planes) => planes.iter(),
        }
    }

    // pub fn flatten(&self) -> Flatten<Iter<PlaneT>> {
    //     self.iter().flatten()
    // }
}

pub trait PlaneListIter {
    fn iter_planes<I>(&self) -> I
    where
        I: Iterator<Item = PlaneT>;
}
pub type PlaneList = Vec<Plane>;

impl PlaneListIter for PlaneList {
    fn iter_planes<I>(&self) -> I
    where
        I: Iterator<Item = PlaneT>,
    {
        self.iter().map(|p| p.iter()).flatten()
    }
}

pub type PropList = Vec<Properties>;

fn plane_from_polygon(poly: PolygonType, layer_index: usize, style_index: usize) -> PlaneT {
    let exterior_ring = &poly[0];
    PlaneT {
        layer_index,
        style_index,
        points: exterior_ring
            .iter()
            .map(|pos| Point::new(pos[0], pos[1], pos[2]))
            .collect(),
    }
}

fn plane_from_geometry(geom: Geometry, layer_index: usize, style_index: usize) -> Plane {
    match geom.value {
        Value::Polygon(v) => Plane::One(plane_from_polygon(v, layer_index, style_index)),
        Value::MultiPolygon(v) => Plane::Multi(
            v.iter()
                .map(|poly| plane_from_polygon(poly.to_vec(), layer_index, style_index))
                .collect(),
        ),
        _ => Plane::None,
    }
}

pub fn plane_from_feature(f: Feature, layer_index: usize, style_index: usize) -> Plane {
    match f.geometry {
        Some(geom) => plane_from_geometry(geom, layer_index, style_index),
        None => Plane::None,
    }
}

// pub fn get_planes(gj: &GeoJson, layer_index: usize) -> PlaneList {
//     match gj {
//         GeoJson::FeatureCollection(fc) => {
//             let mut pl = PlaneList::new();
//             fc.features
//                 .iter()
//                 .for_each(|f| plane_from_feature(f.clone(), &mut pl, layer_index));
//             pl
//         }
//         _ => vec![],
//     }
// }

pub fn get_properties(gj: &GeoJson) -> PropList {
    match gj {
        GeoJson::FeatureCollection(fc) => {
            fc.features.iter().map(|f| f.clone().properties).collect()
        }
        _ => vec![],
    }
}

// pub fn make_cross(pt: Point, cin: f64, cout: f64) -> Plane {
//     let x = pt.x;
//     let y = pt.y;
//     let z = pt.z;
//     vec![
//         Point::new(x - cin, y + cin, z),
//         Point::new(x - cin, y + cout, z),
//         Point::new(x + cin, y + cout, z),
//         Point::new(x + cin, y + cin, z),
//         Point::new(x + cout, y + cin, z),
//         Point::new(x + cout, y - cin, z),
//         Point::new(x + cin, y - cin, z),
//         Point::new(x + cin, y - cout, z),
//         Point::new(x - cin, y - cout, z),
//         Point::new(x - cin, y - cin, z),
//         Point::new(x - cout, y - cin, z),
//         Point::new(x - cout, y + cin, z),
//     ]
// }
