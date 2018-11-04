use geojson::{Error, GeoJson};
use std::fs;

pub fn load_geojson(filename: &str) -> Result<GeoJson, Error> {
    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");

    contents.parse::<GeoJson>()
}
