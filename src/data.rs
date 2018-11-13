use geojson::{Feature, GeoJson, Geometry, PolygonType, Value};
use lingua::get_properties;
use lingua::plane_from_feature;
use lingua::Properties;
use lingua::{Plane, PlaneList};
use serde::de::{self, Deserialize, Deserializer, SeqAccess, Visitor};
use serde_json::Deserializer as JsonDeserializer;

use std::cell::{Ref, RefCell};
use std::fs;
use std::thread;
use std::{cmp, fmt};
use style::{load_style, StyleCollection, StyleList};
use time::precise_time_s;

pub struct Counter {
    name: String,
    count: usize,
    pstep: usize,
    p: usize,
    t: f64,
}

impl Counter {
    pub fn new(name: &str, pstep: usize) -> Counter {
        Counter {
            pstep,
            name: name.to_owned(),
            count: 0,
            p: pstep,
            t: precise_time_s(),
        }
    }

    pub fn inc(&mut self) {
        self.count += 1;
        if self.count == self.p {
            self.p += self.pstep;
            let d = precise_time_s() - self.t;
            println!(
                "Count({}) -> {} {} {}",
                self.name,
                self.count,
                d,
                d / (self.pstep as f64)
            );
            self.t = precise_time_s();
        }
    }
}

#[derive(Clone)]
pub struct DeState {
    pub style: StyleList,
    pub layer_index: usize,
}

thread_local! {
    pub static STATE: RefCell<Option<DeState>> = RefCell::new(None);
}

pub struct Data {
    pub styles: StyleCollection,
    pub planes: PlaneList,
}

#[derive(Deserialize)]
struct FeatureList {
    #[serde(deserialize_with = "deserialize_features")]
    #[serde(rename(deserialize = "features"))]
    planes: PlaneList,
}

fn deserialize_features<'de, D>(deserializer: D) -> Result<PlaneList, D::Error>
where
    D: Deserializer<'de>,
{
    struct FeatureListVisitor;

    impl<'de> Visitor<'de> for FeatureListVisitor {
        type Value = PlaneList;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence of features")
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<PlaneList, S::Error>
        where
            S: SeqAccess<'de>,
        {
            let mut c = Counter::new("visit", 1000);
            let mut planes: PlaneList = PlaneList::new(1000000);

            match STATE.with(|s| s.borrow().clone()) {
                None => (),
                Some(s) => {
                    while let Some(value) = seq.next_element()? {
                        match Feature::from_json_object(value) {
                            Ok(f) => match s.style.select(f.properties.clone()) {
                                Some(style_index) => {
                                    // println!("styled {} {}", s.layer_index, style_index);
                                    c.inc();
                                    planes.push(plane_from_feature(f, s.layer_index, style_index));
                                }
                                None => (),
                            },
                            Err(_) => (),
                        };
                    }
                }
            }

            Ok(planes)
        }
    }

    let visitor = FeatureListVisitor;
    deserializer.deserialize_seq(visitor)
}

impl Data {
    pub fn from_file(filename: &str) -> std::io::Result<Data> {
        let records = std::fs::read_to_string(filename)?;
        let mut planes: PlaneList = PlaneList::new(0);
        let mut styles: StyleCollection = Vec::new();

        records.lines().enumerate().for_each(|(index, r)| {
            let mut file_names: Vec<&str> = r.split(":").collect();
            match file_names.pop() {
                Some(style_fn) => {
                    println!("load_style {}", style_fn);
                    let sj = load_style(style_fn).unwrap();
                    let style = StyleList::from_config(&sj);

                    STATE.with(|s| {
                        *s.borrow_mut() = Some(DeState {
                            style: style.clone(),
                            layer_index: index,
                        })
                    });

                    match file_names.pop() {
                        Some(data_fn) => match ::std::fs::File::open(data_fn) {
                            Ok(f) => {
                                println!("Loading data {}", data_fn);
                                // let deser = serde_json::Deserializer::from_reader(f);
                                // let fl: FeatureList = deser.;
                                let mut r: ::std::result::Result<FeatureList, ::serde_json::Error> = serde_json::from_reader(f);
                                match r {
                                    Ok(ref mut fl) => {
                                        planes.merge(&mut fl.planes);
                                        println!("Loaded");
                                        },
                                    Err(e)=> println!("Error {}", e),
                                }
                            }
                            Err(e) => println!("Error {}", e),
                        },
                        None => (),
                    }

                    styles.push(style);
                }
                None => (),
            }
        });

        Ok(Data { planes, styles })
    }
}
