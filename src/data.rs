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

struct Data {
    planes: PlaneList,
    styles: StyleCollection,
}

pub fn load_layers(filename: &str) -> std::io::Result<Data> {
    let records = std::fs::read_to_string(filename)?;
    let planes: Vec<PlaneList> = Vec::new();
    let styles: StyleCollection = Vec::new();

    records.lines().enumerate().for_each(|(index, r)| {
        let file_names: Vec<&str> = r.split(":").collect();
        file_names.pop().and_then(|data_fn| {
            let gj = load_geojson(data_fn).unwrap();
            let mut pl = get_planes(&gj, index);
            let props = get_properties(&gj);
            file_names.pop().and_then(|style_fn| {
                let sj = load_style(style_fn).unwrap();
                let mut style = StyleList::from_config(&sj);
                style.apply(&props);

                planes.append(&mut pl);
                styles.push(style);

                None
            })
        });
    });

    Ok(Data { planes, styles })
}
