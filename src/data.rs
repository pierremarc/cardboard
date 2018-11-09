use geojson::{Error, GeoJson};
use lingua::get_planes;
use lingua::get_properties;
use lingua::PlaneList;
use std::fs;
use style::{load_style, StyleCollection, StyleList};

pub fn load_geojson(filename: &str) -> Result<GeoJson, Error> {
    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");

    contents.parse::<GeoJson>()
}

pub struct Data {
    pub planes: PlaneList,
    pub styles: StyleCollection,
}

impl Data {
    pub fn from_file(filename: &str) -> std::io::Result<Data> {
        let records = std::fs::read_to_string(filename)?;
        let mut planes: PlaneList = Vec::new();
        let mut styles: StyleCollection = Vec::new();

        records.lines().enumerate().for_each(|(index, r)| {
            let mut file_names: Vec<&str> = r.split(":").collect();

            match file_names.pop() {
                Some(data_fn) => {
                    println!("load_geojson {}", data_fn);
                    let gj = load_geojson(data_fn).unwrap();
                    let mut pl = get_planes(&gj, index);
                    let props = get_properties(&gj);
                    match file_names.pop() {
                        Some(style_fn) => {
                            println!("load_style {}", style_fn);
                            let sj = load_style(style_fn).unwrap();
                            let mut style = StyleList::from_config(&sj);
                            style.apply(&props);

                            planes.append(&mut pl);
                            styles.push(style);
                        }
                        None => (),
                    }
                }
                None => (),
            }
        });

        Ok(Data { planes, styles })
    }
}
