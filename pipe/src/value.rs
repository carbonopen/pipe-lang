use regex::Regex;
use std::{collections::HashMap, ops::Range};

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
    pub handler: String,
}
#[derive(Clone, PartialEq, Debug)]
pub struct Placeholders {
    pub raw: String,
    pub handlers: Vec<Placeholder>,
}

impl Placeholders {
    pub fn from_interpolation(raw: String, handler: String) -> Self {
        Self {
            raw,
            handlers: vec![Placeholder {
                range: Range {
                    start: 0,
                    end: handler.len() - 1,
                },
                handler,
            }],
        }
    }

    pub fn from_string(raw: String) -> Self {
        Self {
            raw: raw.clone(),
            handlers: Placeholders::extract(raw),
        }
    }

    fn extract(raw: String) -> Vec<Placeholder> {
        let re = Regex::new(r"\$\{(?P<handler>.*?)\}").unwrap();
        let mut list = Vec::new();

        for caps in re.captures_iter(&raw) {
            let range = caps.get(0).unwrap().range();
            let handler = caps.get(1).unwrap().as_str().to_string();
            list.push(Placeholder { range, handler })
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

impl Value {
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

    pub fn array_push(&self, target: Value) -> Result<Self, ()> {
        let mut arr = match self.to_array() {
            Ok(mut map) => {
                map.push(target);
                map
            }
            Err(_) => return Err(()),
        };

        Ok(Self::Array(arr))
    }

    pub fn merge_object(&self, target: HashMap<String, Value>) -> Result<Self, ()> {
        let mut obj = match self.to_object() {
            Ok(mut map) => {
                map.extend(target.clone());
                map
            }
            Err(_) => return Err(()),
        };

        Ok(Self::Object(obj.clone()))
    }
}

// pub fn serialize_json(val: &Value) -> String {
//     match val {
//         Value::Object(o) => {
//             let contents: Vec<_> = o
//                 .iter()
//                 .map(|(name, value)| format!("\"{}\":{}", name, serialize_json(value)))
//                 .collect();
//             format!("{{{}}}", contents.join(","))
//         }
//         Value::Array(a) => {
//             let contents: Vec<_> = a.iter().map(serialize_json).collect();
//             format!("[{}]", contents.join(","))
//         }
//         Value::String(s) => format!("\"{}\"", s),
//         Value::Number(n) => format!("{}", n),
//         Value::Boolean(b) => format!("{}", b),
//         Value::Null => format!("null"),
//         Value::Undefined => format!("undefined"),
//         Value::Interpolation(i) => format!("{}", i),
//     }
// }
