use geojson::{Feature, GeoJson, Geometry, PolygonType, Value};
use nalgebra::geometry::{Point2, Point3};
use serde::de::{self, Deserialize, Deserializer, SeqAccess, Visitor};
use serde_json;
use std::iter::Flatten;
use std::iter::FromIterator;
use std::slice::Iter;

pub type Properties = std::option::Option<serde_json::Map<std::string::String, serde_json::Value>>;

pub type Point = Point3<f64>;
pub type Point2D = Point2<f64>;

#[derive(Clone)]
pub struct PlaneT {
    pub layer_index: usize,
    pub style_index: usize,
    pub points: Vec<Point>,
}

// pub enum Plane {
//     None,
//     One(PlaneT),
//     Multi(Vec<PlaneT>),
// }

// static empty_plane: Vec<PlaneT> = Vec::new();

// impl Plane {
//     pub fn iter(&self) -> Iter<PlaneT> {
//         match self {
//             Plane::None => empty_plane.iter(),
//             Plane::One(plane) => vec![*plane].iter(),
//             Plane::Multi(planes) => planes.iter(),
//         }
//     }

//     // pub fn flatten(&self) -> Flatten<Iter<PlaneT>> {
//     //     self.iter().flatten()
//     // }
// }

// pub trait PlaneListIter {
//     fn iter_planes<I>(&self) -> I
//     where
//         I: Iterator<Item = PlaneT>;
// }
const empty_plane: &'static [&'static PlaneT] = &[];

pub type Plane = Vec<PlaneT>;
pub struct PlaneList(Vec<Plane>);
pub type PlaneFlat<'a> = Vec<&'a PlaneT>;

pub struct PlaneIter<'a> {
    outer: Iter<'a, Plane>,
    inner: Iter<'a, PlaneT>,
}

impl PlaneList {
    pub fn new(capacity: usize) -> PlaneList {
        PlaneList(Vec::with_capacity(capacity))
    }

    pub fn flattened(&self) -> PlaneFlat {
        let mut pf: PlaneFlat = Vec::new();
        let planes = &self.0;
        for plane in planes {
            for t in plane {
                pf.push(&t)
            }
        }
        pf
        // self.iter().map(|p| &p).collect()
    }

    pub fn one(&self) -> &Vec<Plane> {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(&mut self, p: Plane) {
        self.0.push(p)
    }

    pub fn merge(&mut self, other: &mut PlaneList) {
        self.0.append(&mut other.0)
    }

    // pub fn iter(&self) -> PlaneIter {
    //     let inner = self
    //         .0
    //         .iter()
    //         .next()
    //         .map_or(empty_plane.iter(), |p| p.iter());

    //     let outer = (&self.0).iter();

    //     PlaneIter { outer, inner }
    // }
}

// impl<'a> Iterator for PlaneIter<'a> {
//     type Item = &'a PlaneT;
//     fn next(&mut self) -> Option<Self::Item> {
//         match self.inner.next() {
//             Some(i) => Some(*i),
//             None => match self.outer.next() {
//                 None => None,
//                 Some(o) => {
//                     self.inner = o.iter();
//                     self.next()
//                 }
//             },
//         }
//     }
// }

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
        Value::Polygon(v) => vec![plane_from_polygon(v, layer_index, style_index)],
        Value::MultiPolygon(v) => Plane::from_iter(
            v.iter()
                .map(|poly| plane_from_polygon(poly.to_vec(), layer_index, style_index)),
        ),
        _ => Vec::new(),
    }
}

pub fn plane_from_feature(f: Feature, layer_index: usize, style_index: usize) -> Plane {
    match f.geometry {
        Some(geom) => plane_from_geometry(geom, layer_index, style_index),
        None => Vec::new(),
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
