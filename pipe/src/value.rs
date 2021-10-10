use std::collections::HashMap;

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
pub enum Value {
    Object(HashMap<String, Value>),
    Array(Vec<Value>),
    String(String),
    Number(String),
    Interpolation(String),
    Boolean(bool),
    Null,
}

impl Value {
    pub(crate) fn set_string_from_string(value: &str) -> Self {
        let mut chars = value.chars();
        chars.next();
        chars.next_back();
        Self::String(chars.as_str().to_string())
    }

    pub fn to_object(self) -> Result<HashMap<String, Value>, ()> {
        match self {
            Self::Object(value) => Ok(value),
            _ => Err(()),
        }
    }
}

pub fn serialize_json(val: &Value) -> String {
    match val {
        Value::Object(o) => {
            let contents: Vec<_> = o
                .iter()
                .map(|(name, value)| format!("\"{}\":{}", name, serialize_json(value)))
                .collect();
            format!("{{{}}}", contents.join(","))
        }
        Value::Array(a) => {
            let contents: Vec<_> = a.iter().map(serialize_json).collect();
            format!("[{}]", contents.join(","))
        }
        Value::String(s) => format!("\"{}\"", s),
        Value::Number(n) => format!("{}", n),
        Value::Boolean(b) => format!("{}", b),
        Value::Null => format!("null"),
        Value::Interpolation(i) => format!("{}", i),
    }
}
