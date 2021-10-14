use regex::Regex;
use std::{collections::HashMap, ops::Range};

use crate::Value::Boolean;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Number {
    pub value: String,
}

#[derive(Clone, Eq, PartialEq, Debug)]

pub struct Interpolation {
    pub value: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Object {
    pub value: HashMap<String, Value>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Placeholder {
    pub range: Range<usize>,
    pub script: String,
}
#[derive(Clone, PartialEq, Debug)]
pub struct Placeholders {
    pub raw: String,
    pub scripts: Vec<Placeholder>,
}

impl Placeholders {
    pub fn from_interpolation(raw: String, script: String) -> Self {
        Self {
            raw,
            scripts: vec![Placeholder {
                range: Range {
                    start: 0,
                    end: script.len() - 1,
                },
                script,
            }],
        }
    }

    pub fn from_string(raw: String) -> Self {
        Self {
            raw: raw.clone(),
            scripts: Placeholders::extract(raw),
        }
    }

    fn extract(raw: String) -> Vec<Placeholder> {
        let re = Regex::new(r"\$\{(?P<handler>.*?)\}").unwrap();
        let mut list = Vec::new();

        for caps in re.captures_iter(&raw) {
            let range = caps.get(0).unwrap().range();
            let script = caps.get(1).unwrap().as_str().to_string();
            list.push(Placeholder { range, script })
        }

        list
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Object(HashMap<String, Value>),
    Array(Vec<Value>),
    String(String),
    Number(String),
    Interpolation(Placeholders),
    Boolean(bool),
    Null,
    Undefined,
}

#[cfg(feature = "comparators")]
impl Value {
    pub fn is_boolean(&self) -> bool {
        match self {
            Self::Boolean(_) => true,
            _ => false,
        }
    }

    pub fn is_object(&self) -> bool {
        match self {
            Self::Object(_) => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            Self::Array(_) => true,
            _ => false,
        }
    }

    pub fn is_string(&self) -> bool {
        match self {
            Self::String(_) => true,
            _ => false,
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            Self::Number(_) => true,
            _ => false,
        }
    }

    pub fn is_interpolation(&self) -> bool {
        match self {
            Self::Interpolation(_) => true,
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            Self::Null => true,
            _ => false,
        }
    }

    pub fn is_undefined(&self) -> bool {
        match self {
            Self::Undefined => true,
            _ => false,
        }
    }
}

#[cfg(feature = "converters")]
impl Value {
    pub fn to_boolean(&self) -> Result<bool, ()> {
        match self {
            Self::Boolean(value) => Ok(value.clone()),
            _ => Err(()),
        }
    }

    pub fn to_string(&self) -> Result<String, ()> {
        match self {
            Self::String(value) => Ok(value.clone()),
            Self::Number(value) => Ok(value.clone()),
            Self::Interpolation(value) => Ok(value.raw.clone()),
            Self::Null => Ok("null".to_string()),
            Self::Undefined => Ok("undefined".to_string()),
            _ => Err(()),
        }
    }

    pub fn to_f64(&self) -> Result<f64, ()> {
        match self {
            Self::Number(value) => match value.parse::<f64>() {
                Ok(value) => Ok(value),
                Err(_) => Err(()),
            },
            _ => Err(()),
        }
    }

    pub fn to_i64(&self) -> Result<i64, ()> {
        match self {
            Self::Number(value) => match value.parse::<i64>() {
                Ok(value) => Ok(value),
                Err(_) => Err(()),
            },
            _ => Err(()),
        }
    }

    pub fn to_object(&self) -> Result<HashMap<String, Value>, ()> {
        match self {
            Self::Object(value) => Ok(value.clone()),
            _ => Err(()),
        }
    }

    pub fn to_array(&self) -> Result<Vec<Value>, ()> {
        match self {
            Self::Array(value) => Ok(value.clone()),
            _ => Err(()),
        }
    }

    pub fn to_placeholders(&self) -> Result<Placeholders, ()> {
        match self {
            Self::Interpolation(value) => Ok(value.clone()),
            _ => Err(()),
        }
    }

    pub fn array_push(&self, target: Value) -> Result<Self, ()> {
        let arr = match self.to_array() {
            Ok(mut map) => {
                map.push(target);
                map
            }
            Err(_) => return Err(()),
        };

        Ok(Self::Array(arr))
    }

    pub fn merge_object(&self, target: HashMap<String, Value>) -> Result<Self, ()> {
        let obj = match self.to_object() {
            Ok(mut map) => {
                map.extend(target.clone());
                map
            }
            Err(_) => return Err(()),
        };

        Ok(Self::Object(obj.clone()))
    }
}

#[cfg(feature = "json")]
impl Value {
    pub fn as_json(&self) -> String {
        Value::to_json(self)
    }

    pub fn to_json(val: &Value) -> String {
        match val {
            Value::Object(o) => {
                let contents: Vec<_> = o
                    .iter()
                    .map(|(name, value)| format!("\"{}\":{}", name, Value::to_json(value)))
                    .collect();
                format!("{{{}}}", contents.join(","))
            }
            Value::Array(a) => {
                let contents: Vec<_> = a.iter().map(Value::to_json).collect();
                format!("[{}]", contents.join(","))
            }
            Value::String(s) => format!("\"{}\"", s),
            Value::Number(n) => format!("{}", n),
            Value::Boolean(b) => format!("{}", b),
            Value::Null => format!("null"),
            Value::Undefined => format!("undefined"),
            Value::Interpolation(place) => {
                let mut map = HashMap::new();
                map.insert("raw".to_string(), Value::String(place.raw.clone()));

                let scripts = {
                    let mut list = Vec::new();

                    for scr in place.scripts.clone() {
                        let mut map = HashMap::new();
                        map.insert("script".to_string(), Value::String(scr.script.clone()));
                        map.insert(
                            "start".to_string(),
                            Value::Number(scr.range.start.to_string()),
                        );
                        map.insert("end".to_string(), Value::Number(scr.range.end.to_string()));
                        list.push(Value::Object(map));
                    }

                    Value::Array(list)
                };

                map.insert("scripts".to_string(), scripts);

                format!("{}", Value::to_json(&Value::Object(map)))
            }
        }
    }
}
