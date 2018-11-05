use serde_json;
use std::fs;
use svgtypes::Color as CSSColor;

pub type Properties = std::option::Option<serde_json::Map<std::string::String, serde_json::Value>>;

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

pub struct StyleList(Vec<Style>, Option<Vec<usize>>);

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

    pub fn get_for(&self, index: usize) -> Option<&Style> {
        let styles = &self.0;
        match self.1 {
            Some(ref apply_list) => {
                apply_list
                    .get(index)
                    .and_then(|style_index| styles.get(style_index.to_owned()))

                // if index < apply_list.len() {
                //     let i = apply_list[index];
                //     let s = self.0[];
                //     Some(s.clone())
                // } else {
                //     None
                // }
            }
            _ => None,
        }
    }

    pub fn from_config(style_config: &PolygonStyleConfig) -> StyleList {
        let mut sl = StyleList::new();
        match style_config {
            PolygonStyleConfig::Simple(config) => {
                sl.add(
                    Style::new(StyleConfig::Simple)
                        .width(config.strokeWidth)
                        .stroke(Color::from_string(&config.strokeColor))
                        .fill(Color::from_string(&config.fillColor)),
                );
            }
            PolygonStyleConfig::Continuous(config) => {
                config.intervals.iter().for_each(|it| {
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

    pub fn apply(&mut self, props: &Vec<Properties>) -> &StyleList {
        let mut apply_list: Vec<usize> = props
            .iter()
            .map(|properties| {
                self.0
                    .iter()
                    .position(|s| match s.config.clone() {
                        StyleConfig::Simple => true,

                        StyleConfig::Continuous(config) => {
                            properties.clone().map_or(false, |props| {
                                props.get(&config.prop_name).map_or(false, |v| {
                                    v.as_f64()
                                        .map_or(false, |n| n >= config.low && n < config.high)
                                })
                            })
                        }

                        StyleConfig::Discrete(config) => {
                            properties.clone().map_or(false, |props| {
                                props.get(&config.prop_name).map_or(false, |v| {
                                    v.as_str()
                                        .map_or(false, |s| config.toks.iter().any(|t| t == s))
                                })
                            })
                        }
                    }).get_or_insert(0)
                    .to_owned()
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
pub enum PolygonStyleConfig {
    Simple(PolygonStyleConfigSimple),
    Continuous(PolygonStyleConfigContinuous),
    Discrete(PolygonStyleConfigDiscrete),
}

pub fn load_style(filename: &str) -> Result<PolygonStyleConfig, serde_json::Error> {
    let serialized = fs::read_to_string(filename).expect("Something went wrong reading the file");

    serde_json::from_str::<PolygonStyleConfig>(&serialized)
}