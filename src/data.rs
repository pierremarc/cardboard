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

// pub fn load_geojson(filename: &str) -> Result<GeoJson, Error> {
//     let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");

//     contents.parse::<GeoJson>()
// }

#[derive(Clone)]
pub struct DeState {
    pub style: StyleList,
    pub layer_index: usize,
}

// #[derive(Clone)]
// pub enum State {
//     None,
//     S(DeState),
// }

thread_local! {
    pub static STATE: RefCell<Option<DeState>> = RefCell::new(None);
}

// struct FeatureVisitor {
//     style: StyleList,
// }

// impl<'de> Visitor<'de> for FeatureVisitor {
//     type Value = Plane;

//     fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//         formatter.write_str("a geojson feature")
//     }

//     #[inline]
//     fn visit_unit<E>(self) -> Result<Self::Value, E>
//     where
//         E: de::Error,
//     {
//         Ok(Plane::None)
//     }

//     #[inline]
//     fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
//     where
//         V: de::MapAccess<'de>,
//     {
//         let mut values = ::serde_json::map::Map::new();

//         while let Some((key, value)) = try!(visitor.next_entry()) {
//             values.insert(key, value);
//         }

//         match Feature::from_json_object(values) {
//             Ok(f) => match self.style.select(f.properties) {
//                 Some(style_index) => Ok(plane_from_feature2(f, 0, style_index)),
//                 None => Ok(Plane::None),
//             },
//             Err(_) => Ok(Plane::None),
//         }
//     }
// }

trait Styled {
    fn get_style_list(&self) -> StyleList;
}

pub struct Data {
    pub styles: StyleCollection,
    pub planes: PlaneList,
}

// pub struct DataDeserializer<R>
// where
//     R: std::io::Read,
// {
//     deser: JsonDeserializer<::serde_json::de::IoRead<R>>,
//     pub style: StyleList,
//     pub current_layer_index: usize,
// }

// impl<'de, R> DataDeserializer<R>
// where
//     R: ::std::io::Read,
// {
//     pub fn new(style: StyleList, current_layer_index: usize, r: R) -> Self {
//         DataDeserializer {
//             deser: JsonDeserializer::from_reader(r),
//             style,
//             current_layer_index,
//         }
//     }

//     fn deserialize_feature(
//         self,
//         visitor: FeatureVisitor,
//     ) -> Result<Plane, ::serde_json::error::Error> {
//         visitor.visit_feature(serde_json::map::MapAccess::new(self), self.style)
//     }
// }

// impl<'de, R> Deserializer<'de> for DataDeserializer<R>
// where
//     R: std::io::Read,
// {
//     type Error = ::serde_json::error::Error;

//     fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: Visitor<'de>,
//     {
//         self.deser.deserialize_any(visitor)
//     }

//     fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: Visitor<'de>,
//     {
//         self.deser.deserialize_map(visitor)
//     }

//     forward_to_deserialize_any! {
//         bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
//         bytes byte_buf option unit unit_struct newtype_struct seq tuple
//         tuple_struct struct enum identifier ignored_any
//     }
// }

// impl<'de, R> Styled for DataDeserializer<R>
// where
//     R: std::io::Read,
// {
//     fn get_style_list(&self) -> StyleList {
//         self.style
//     }
// }

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
            let mut planes: PlaneList = PlaneList::new();

            match STATE.with(|s| s.borrow().clone()) {
                None => (),
                Some(s) => {
                    while let Some(value) = seq.next_element()? {
                        match Feature::from_json_object(value) {
                            Ok(f) => match s.style.select(f.properties.clone()) {
                                Some(style_index) => {
                                    println!("styled {} {}", s.layer_index, style_index);
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
        let mut planes: PlaneList = PlaneList::new();
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
