use geojson::{Feature, GeoJson, Geometry, PolygonType, Value};
use nalgebra::geometry::{Point2, Point3};
use serde_json;

pub type Properties = std::option::Option<serde_json::Map<std::string::String, serde_json::Value>>;

pub type Point = Point3<f64>;
pub type Point2D = Point2<f64>;
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

pub fn get_planes(gj: &GeoJson) -> PlaneList {
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

pub fn get_properties(gj: &GeoJson) -> PropList {
    match gj {
        GeoJson::FeatureCollection(fc) => {
            fc.features.iter().map(|f| f.clone().properties).collect()
        }
        _ => vec![],
    }
}

pub fn make_cross(pt: Point, cin: f64, cout: f64) -> Plane {
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
