use regex::Regex;
use std::{collections::HashMap, ops::Range};
use uuid::Uuid;

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

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Script {
    pub raw: String,
    pub groups: Vec<String>,
}

impl Script {
    pub fn from_interpolation(script: String) -> Self {
        Self {
            groups: vec![script.trim().to_string()],
            raw: script,
        }
    }

    pub fn from_string(raw: String) -> Self {
        let groups = Self::extract(&raw);

        Self { groups, raw }
    }

    fn extract(raw: &String) -> Vec<String> {
        let re = Regex::new(r"(\$\{(?P<script>.*?)\})").unwrap();
        let mut list = Vec::new();
        let mut pos: usize = 0;

        for caps in re.captures_iter(&raw) {
            let range = caps.get(0).unwrap().range();
            let script = caps.get(1).unwrap().as_str().to_string();
            list.push(format!(r#""{}""#, raw[pos..range.start].to_string()));
            list.push(script[2..(script.len() - 1)].trim().to_string());
            pos = range.end;
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
    Interpolation(Script),
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
            Self::Null => Ok(false),
            Self::Undefined => Ok(false),
            Self::Object(_) => Ok(true),
            Self::Array(_) => Ok(true),
            Self::String(_) => Ok(true),
            Self::Number(_) => Ok(true),
            Self::Interpolation(_) => Ok(true),
        }
    }

    pub fn to_string(&self) -> Result<String, ()> {
        match self {
            Self::String(value) => Ok(value.clone()),
            Self::Number(value) => Ok(value.clone()),
            Self::Interpolation(value) => Ok(value.raw.clone()),
            Self::Boolean(value) => Ok(format!("{}", value)),
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

    pub fn to_script(&self) -> Result<Script, ()> {
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
            Value::Interpolation(script) => {
                let mut map = HashMap::new();
                map.insert(
                    "__type".to_string(),
                    Value::String("interpolation".to_string()),
                );
                map.insert("__raw".to_string(), Value::String(script.raw.clone()));

                let groups = script
                    .groups
                    .iter()
                    .map(|item| Value::String(item.clone()))
                    .collect::<Vec<_>>();
                map.insert("__groups".to_string(), Value::Array(groups));

                format!("{}", Value::to_json(&Value::Object(map)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::map;

    use super::*;

    #[test]
    fn comparators() {
        assert_eq!(Value::Array(Vec::default()).is_array(), true);
        assert_eq!(Value::Object(HashMap::default()).is_object(), true);
        assert_eq!(Value::String(String::default()).is_string(), true);
        assert_eq!(Value::Number("1".to_string()).is_number(), true);
        assert_eq!(
            Value::Interpolation(Script::default()).is_interpolation(),
            true
        );
        assert_eq!(Value::Boolean(false).is_boolean(), true);
        assert_eq!(Value::Null.is_null(), true);
        assert_eq!(Value::Undefined.is_undefined(), true);
    }
    #[test]
    fn converters() {
        let array = Value::Array(Vec::default());
        let object = Value::Object(HashMap::default());
        let string = Value::String("".to_string());
        let number = Value::Number("1".to_string());
        let boolean = Value::Boolean(true);
        let null = Value::Null;
        let undefined = Value::Undefined;
        let interpolation = Value::Interpolation(Script::default());

        assert_eq!(array.to_array().unwrap(), Vec::default());
        assert_eq!(array.to_boolean().unwrap(), true);

        assert_eq!(object.to_object().unwrap(), HashMap::default());
        assert_eq!(object.to_boolean().unwrap(), true);

        assert_eq!(string.to_string().unwrap(), "".to_string());
        assert_eq!(string.to_boolean().unwrap(), true);

        assert_eq!(number.to_string().unwrap(), "1".to_string());
        assert_eq!(number.to_boolean().unwrap(), true);
        assert_eq!(number.to_i64().unwrap(), 1);

        assert_eq!(boolean.to_string().unwrap(), "true".to_string());
        assert_eq!(boolean.to_boolean().unwrap(), true);

        assert_eq!(null.to_string().unwrap(), "null".to_string());
        assert_eq!(null.to_boolean().unwrap(), false);

        assert_eq!(undefined.to_string().unwrap(), "undefined".to_string());
        assert_eq!(undefined.to_boolean().unwrap(), false);

        assert_eq!(interpolation.to_string().unwrap(), "".to_string());
        assert_eq!(interpolation.to_boolean().unwrap(), true);
        assert_eq!(interpolation.to_script().unwrap(), Script::default());

        assert_eq!(Value::Number("1.5".to_string()).to_f64().unwrap(), 1.5);
    }

    #[test]
    fn as_json() {
        let map = map!("item".to_string(), Value::Boolean(true));
        let object = Value::Object(map);
        assert_eq!(object.as_json(), "{\"item\":true}".to_string());
    }
}
