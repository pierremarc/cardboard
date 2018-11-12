use lingua::Properties;
use serde_json;
use std::fs;
use svgtypes::Color as CSSColor;

#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

#[derive(Clone, Debug)]
pub struct StyleConfigContinuous {
    prop_name: String,
    low: f64,
    high: f64,
}

#[derive(Clone, Debug)]
pub struct StyleConfigDiscrete {
    prop_name: String,
    toks: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum StyleConfig {
    Simple,
    Continuous(StyleConfigContinuous),
    Discrete(StyleConfigDiscrete),
}

#[derive(Clone, Debug)]
pub struct Style {
    pub strokeWidth: f64,
    pub strokeColor: Option<Color>,
    pub fillColor: Option<Color>,
    config: StyleConfig,
}

// pub enum StyleListKind {
//     Simple,
//     Continuous,
//     Discrete,
// }

#[derive(Clone, Debug)]
pub struct StyleList(Vec<Style>, Option<Vec<usize>>);

pub type StyleCollection = Vec<StyleList>;

pub trait StyleGetter {
    fn get_for(&self, list_index: &usize, style_index: &usize) -> Option<&Style>;
}

impl StyleGetter for StyleCollection {
    fn get_for(&self, list_index: &usize, style_index: &usize) -> Option<&Style> {
        self.get(list_index.to_owned())
            .and_then(|style_list| style_list.get_for(style_index))
    }
}

fn u2f(v: u8) -> f64 {
    let vf = v as f64;
    vf / 255.0
}

impl Color {
    pub fn new() -> Color {
        Color {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            alpha: 1.0,
        }
    }

    pub fn from_string(s: &str) -> Color {
        match s.parse::<CSSColor>() {
            Ok(css_color) => Color::rgb(
                u2f(css_color.red),
                u2f(css_color.green),
                u2f(css_color.blue),
            ),
            Err(_) => Color::new(),
        }
    }

    pub fn rgb(red: f64, green: f64, blue: f64) -> Color {
        Color {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    pub fn rgba(red: f64, green: f64, blue: f64, alpha: f64) -> Color {
        Color {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub fn white() -> Color {
        Color::new()
    }
    pub fn black() -> Color {
        Color::rgb(0.0, 0.0, 0.0)
    }
}

impl Style {
    pub fn new(config: StyleConfig) -> Style {
        Style {
            strokeWidth: 1.0,
            strokeColor: None,
            fillColor: None,
            config,
        }
    }

    pub fn default() -> Style {
        Style {
            strokeWidth: 1.0,
            strokeColor: Some(Color::black()),
            fillColor: Some(Color::white()),
            config: StyleConfig::Simple,
        }
    }

    // pub fn clone(&self) -> Style {
    //     Style{
    //         strokeWidth. sel.strokeWidth,
    //         strokeColor: self.strokeColor,
    //         fillColor: self.fillColor,
    //         config: self.config,
    //     }
    // }

    pub fn width(self, strokeWidth: f64) -> Style {
        Style {
            strokeWidth,
            strokeColor: self.strokeColor,
            fillColor: self.fillColor,
            config: self.config,
        }
    }
    pub fn stroke(self, strokeColor: Color) -> Style {
        Style {
            strokeWidth: self.strokeWidth,
            strokeColor: Some(strokeColor),
            fillColor: self.fillColor,
            config: self.config,
        }
    }
    pub fn fill(self, fillColor: Color) -> Style {
        Style {
            strokeWidth: self.strokeWidth,
            strokeColor: self.strokeColor,
            fillColor: Some(fillColor),
            config: self.config,
        }
    }
}

impl StyleList {
    pub fn new() -> StyleList {
        StyleList(Vec::new(), None)
    }

    fn add(&mut self, s: Style) -> &mut StyleList {
        self.0.push(s);
        self
    }

    pub fn get_for(&self, index: &usize) -> Option<&Style> {
        let styles = &self.0;
        match self.1 {
            Some(ref apply_list) => apply_list
                .get(index.to_owned())
                .and_then(|style_index| styles.get(style_index.to_owned())),
            _ => None,
        }
    }

    pub fn from_config(style_config: &PolygonStyleConfig) -> StyleList {
        let mut sl = StyleList::new();
        match style_config {
            PolygonStyleConfig::Simple(config) => {
                // println!("Insert Simple Style {:?}", config);
                sl.add(
                    Style::new(StyleConfig::Simple)
                        .width(config.strokeWidth)
                        .stroke(Color::from_string(&config.strokeColor))
                        .fill(Color::from_string(&config.fillColor)),
                );
            }
            PolygonStyleConfig::Continuous(config) => {
                config.intervals.iter().for_each(|it| {
                    // println!("Insert Continuous Style {:?}", it);
                    sl.add(
                        Style::new(StyleConfig::Continuous(StyleConfigContinuous {
                            prop_name: config.propName.clone(),
                            high: it.high.clone(),
                            low: it.low.clone(),
                        })).width(it.strokeWidth)
                        .stroke(Color::from_string(&it.strokeColor))
                        .fill(Color::from_string(&it.fillColor)),
                    );
                });
            }
            PolygonStyleConfig::Discrete(config) => {
                config.groups.iter().for_each(|it| {
                    // println!("Insert Discrete Style {:?}", it);
                    sl.add(
                        Style::new(StyleConfig::Discrete(StyleConfigDiscrete {
                            prop_name: config.propName.clone(),
                            toks: it.values.clone(),
                        })).width(it.strokeWidth)
                        .stroke(Color::from_string(&it.strokeColor))
                        .fill(Color::from_string(&it.fillColor)),
                    );
                });
            }
        };
        // make sure ther's a default style at the end
        sl.add(Style::default());
        sl
    }

    pub fn select(&self, props_opt: Properties) -> Option<usize> {
        let style_iterator = self.0.iter();
        style_iterator.position(|s| match s.config {
            StyleConfig::Simple => true,

            StyleConfig::Continuous(config) => props_opt.map_or(false, |props| {
                props.get(&config.prop_name).map_or(false, |v| {
                    v.as_f64()
                        .map_or(false, |n| n >= config.low && n < config.high)
                })
            }),

            StyleConfig::Discrete(config) => props_opt.map_or(false, |props| {
                props.get(&config.prop_name).map_or(false, |v| {
                    v.as_str()
                        .map_or(false, |s| config.toks.iter().any(|t| t == s))
                })
            }),
        })
    }

    pub fn apply(&mut self, props: &Vec<Properties>) -> &StyleList {
        let apply_list: Vec<usize> = props
            .iter()
            .map(|properties| {
                match self.0.iter().position(|s| match s.config.clone() {
                    StyleConfig::Simple => true,

                    StyleConfig::Continuous(config) => properties.clone().map_or(false, |props| {
                        props.get(&config.prop_name).map_or(false, |v| {
                            v.as_f64().map_or(false, |n| {
                                // println!(
                                //     // "StyleConfig::Continuous {} {} {} => {}",
                                //     n,
                                //     config.low,
                                //     config.high,
                                //     n >= config.low && n < config.high,
                                // );
                                n >= config.low && n < config.high
                            })
                        })
                    }),

                    StyleConfig::Discrete(config) => properties.clone().map_or(false, |props| {
                        props.get(&config.prop_name).map_or(false, |v| {
                            v.as_str()
                                .map_or(false, |s| config.toks.iter().any(|t| t == s))
                        })
                    }),
                }) {
                    Some(i) => i.to_owned(),
                    None => panic!("Could not find a style"),
                }
            }).collect();

        self.1 = Some(apply_list);
        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PolygonStyleConfigSimple {
    kind: String,
    strokeColor: String,
    fillColor: String,
    strokeWidth: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PolygonInterval {
    low: f64,
    high: f64,
    fillColor: String,
    strokeColor: String,
    strokeWidth: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PolygonStyleConfigContinuous {
    kind: String,
    propName: String,
    intervals: Vec<PolygonInterval>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PolygonGroup {
    values: Vec<String>,
    fillColor: String,
    strokeColor: String,
    strokeWidth: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PolygonStyleConfigDiscrete {
    kind: String,
    propName: String,
    groups: Vec<PolygonGroup>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum PolygonStyleConfig {
    #[serde(rename = "simple")]
    Simple(PolygonStyleConfigSimple),
    #[serde(rename = "continuous")]
    Continuous(PolygonStyleConfigContinuous),
    #[serde(rename = "discrete")]
    Discrete(PolygonStyleConfigDiscrete),
}

pub fn load_style(filename: &str) -> Result<PolygonStyleConfig, serde_json::Error> {
    let serialized = fs::read_to_string(filename).expect("Something went wrong reading the file");
    // println!("{}", serialized);

    serde_json::from_str::<PolygonStyleConfig>(&serialized)
}

#[cfg(test)]
mod tests {
    use style;
    #[test]
    fn parse_simple() {
        let s = r#"{"kind":"simple","strokeColor":"red","fillColor":"blue","strokeWidth":2.0}"#;
        let p = style::PolygonStyleConfig::Simple(style::PolygonStyleConfigSimple {
            kind: "simple".to_owned(),
            strokeColor: "red".to_owned(),
            fillColor: "blue".to_owned(),
            strokeWidth: 2.0,
        });
        serde_json::to_string(&p).map(|ser| {
            println!("{}", ser);
            assert_eq!(s, ser);
        });
    }
}
