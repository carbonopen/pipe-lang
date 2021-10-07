#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Number {
    n: String,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Value<'a> {
    Object(Vec<(&'a str, Value<'a>)>),
    Array(Vec<Value<'a>>),
    String(&'a str),
    Number(f64),
    Boolean(bool),
    Null,
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
    }
}
