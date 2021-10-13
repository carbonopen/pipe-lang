extern crate pest;
#[macro_use]
extern crate pest_derive;
mod value;

use pest::error::Error as PestError;
use pest::iterators::Pair;
use pest::Parser;
use std::collections::HashMap;
use std::fs;
use value::Value;

use crate::value::Placeholders;
#[derive(Parser)]
#[grammar = "pipe.pest"]
struct PipeParser;

#[derive(Debug)]
pub struct Pipe {}

macro_rules! map {
    () => {
        HashMap::new();
    };
    ($key:expr, $value:expr) => {{
        let mut map = map!();
        map.insert($key, $value);
        map
    }};
}

#[derive(Debug)]
pub struct Error {
    parse: Option<PestError<Rule>>,
    local: Option<String>,
}

impl Error {
    pub fn parse(error: PestError<Rule>) -> Self {
        Self {
            parse: Some(error),
            local: None,
        }
    }

    pub fn local(local: &str) -> Self {
        Self {
            parse: None,
            local: Some(local.to_string()),
        }
    }
}

impl Pipe {
    pub fn from_str(unparsed_file: &str) -> Result<Value, Error> {
        match PipeParser::parse(Rule::pipe, unparsed_file) {
            Ok(mut pairs) => match pairs.next() {
                Some(pair) => Ok(Self::parse(pair)),
                None => Ok(Value::Undefined),
            },
            Err(error) => Err(Error::parse(error)),
        }
    }

    pub fn from_path(path: &str) -> Result<Value, Error> {
        let unparsed_file = match fs::read_to_string(path) {
            Ok(file) => file,
            Err(err) => return Err(Error::local(&err.to_string())),
        };

        Self::from_str(&unparsed_file)
    }

    fn parse(pair: Pair<Rule>) -> Value {
        match pair.as_rule() {
            Rule::sessions => {
                let mut item: HashMap<String, Value> = map!();

                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::session_generic_config => {
                            let mut inner = pair.into_inner();
                            let name = inner.next().unwrap().as_str().to_string();
                            let value = Self::parse(inner.next().unwrap());

                            match item.get(&name) {
                                Some(cur_value) => {
                                    let value =
                                        cur_value.merge_object(value.to_object().unwrap()).unwrap();
                                    item.insert(name, value);
                                }
                                None => {
                                    item.insert(name, value);
                                }
                            };
                        }
                        Rule::session_pipeline => {
                            let mut inner = pair.into_inner();
                            let name = inner.next().unwrap().as_str().to_string();

                            let value = Self::parse(inner.next().unwrap());

                            match item.get(&name) {
                                Some(cur_value) => {
                                    let mut new_value = cur_value.to_array().unwrap();
                                    new_value.extend(value.to_array().unwrap());
                                    item.insert(name, Value::Array(new_value));
                                }
                                None => {
                                    item.insert(name, value);
                                }
                            };
                        }
                        _ => {}
                    }
                }

                Value::Object(item)
            }
            Rule::session_generic_config_content => {
                // common_content
                Self::parse(pair.into_inner().next().unwrap())
            }
            Rule::session_pipeline_content => {
                let mut list = Vec::new();

                for pair in pair.into_inner() {
                    list.push(Self::parse(pair));
                }

                Value::Array(list)
            }
            Rule::module => {
                let mut inner = pair.into_inner();
                let module_name = Value::String(inner.next().unwrap().as_str().to_string());
                let params = Self::parse(inner.next().unwrap());

                let mut map = map!("module".to_string(), module_name);
                map.extend(params.to_object().unwrap());

                Value::Object(map)
            }
            Rule::module_content => {
                let mut map = map!();

                for pair in pair.into_inner() {
                    let rule = pair.as_rule();
                    let value = Self::parse(pair);

                    match rule {
                        Rule::attach => {
                            let value = map!("attach".to_string(), value);
                            map.extend(value);
                        }
                        _ => {
                            let value = match map.get("params") {
                                Some(cur_value) => {
                                    cur_value.merge_object(value.to_object().unwrap()).unwrap();

                                    map!("params".to_string(), cur_value.clone())
                                }
                                None => {
                                    map!("params".to_string(), value)
                                }
                            };

                            map.extend(value);
                        }
                    };
                }

                Value::Object(map)
            }
            Rule::attach => {
                let mut inner = pair.into_inner();
                let value = Self::parse(inner.next().unwrap());

                value
            }
            Rule::param_macro_content => {
                // common_content
                match pair.into_inner().next() {
                    Some(pair) => Self::parse(pair),
                    None => Value::Object(map!()),
                }
            }
            Rule::common_content => {
                let mut map = map!();

                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::param => {
                            let value = Self::make_param(pair);
                            map.insert(value.0, value.1);
                        }
                        Rule::param_macro => {
                            let (name, value) = Self::make_param_macro(pair);

                            match map.get(&name) {
                                Some(cur_value) => {
                                    let value = cur_value.array_push(value).unwrap();
                                    map.insert(name, value);
                                }
                                None => {
                                    map.insert(name, Value::Array(vec![value]));
                                }
                            }
                        }
                        _ => {
                            map.insert("".to_string(), Value::Undefined);
                        }
                    };
                }

                Value::Object(map)
            }
            Rule::object => {
                let mut map = map!();

                for pair in pair.into_inner() {
                    let mut inner = pair.into_inner();
                    let key = inner.next().unwrap().as_str().to_string();
                    let value = Self::parse(inner.next().unwrap());

                    map.insert(key, value);
                }

                Value::Object(map)
            }
            Rule::number => Value::Number(pair.as_str().to_string()),
            Rule::boolean => {
                let value = if pair.as_str().eq("true") {
                    true
                } else {
                    false
                };

                Value::Boolean(value)
            }
            Rule::string => {
                let mut inner = pair.clone().into_inner();
                Self::parse(inner.next().unwrap())
            }
            Rule::string_content => Value::String(pair.as_str().to_string()),
            Rule::ident => Value::String(pair.as_str().to_string()),
            Rule::string_interpolation => {
                let mut inner = pair.clone().into_inner();
                Self::parse(inner.next().unwrap())
            }
            Rule::string_interpolation_content => {
                let raw = pair.as_str().to_string();
                Value::Interpolation(Placeholders::from_string(raw))
            }
            Rule::interpolation => {
                let raw = pair.as_str().to_string();
                let mut inner = pair.into_inner();
                let value = inner.next().unwrap().as_str().trim().to_string();

                Value::Interpolation(Placeholders::from_interpolation(raw, value))
            }
            _ => Value::Undefined,
        }
    }

    fn make_param(pair: Pair<Rule>) -> (String, Value) {
        let mut inner = pair.into_inner();
        let key = inner.next().unwrap().as_str().to_string();
        let value = Self::parse(inner.next().unwrap());

        (key, value)
    }

    fn make_param_macro(pair: Pair<Rule>) -> (String, Value) {
        let mut inner = pair.into_inner();
        let mut map = map!();
        let key = inner.next().unwrap().as_str().to_string();
        let attr2 = inner.next().unwrap();

        match attr2.as_rule() {
            Rule::param_macro_content => {
                let params = Self::parse(attr2);
                (key, params)
            }
            _ => {
                let value = Self::parse(attr2);
                map.insert(key.clone(), value);
                let params = Self::parse(inner.next().unwrap());
                (key, params.merge_object(map).unwrap())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let pipe = r#"
            pipeline {
                macro "http" (
                    test=true
                )
            }
        "#;
        let value = Pipe::from_str(pipe);
        assert!(value.is_ok());
    }

    #[test]
    fn parse_json() {
        if !cfg!(feature = "json") {
            return assert!(true);
        }

        let pipe = r#"
            pipeline {
                macro "http" (
                    test=true
                )
            }
        "#;
        let value = Pipe::from_str(pipe).unwrap();
        assert!(value.as_json().contains("pipeline"));
    }

    #[test]
    fn import_file() {
        let value = Pipe::from_path("../demo/example.pipe");
        assert!(value.is_ok());
    }
}
