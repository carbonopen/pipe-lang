extern crate pest;
#[macro_use]
extern crate pest_derive;
mod value;

use pest::iterators::Pair;
use pest::Parser;
use std::collections::HashMap;
use std::hash::Hash;
use std::{fs, vec};
use value::Value;

use crate::value::{serialize_json, Placeholders};
#[derive(Parser)]
#[grammar = "pipe.pest"]
pub struct PipeParser;

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

fn pipe() {
    let unparsed_file = fs::read_to_string("../demo/example.pipe").expect("cannot read file");

    let mut pair = PipeParser::parse(Rule::pipe, &unparsed_file)
        .unwrap()
        .next()
        .unwrap();

    let json = parse(pair);

    println!("");
    println!("RESULT: {:#?}", json);
    println!("RESULT: {:#?}", serialize_json(&json));
}

fn make_param(pair: Pair<Rule>) -> (String, Value) {
    let mut inner = pair.into_inner();
    let key = inner.next().unwrap().as_str().to_string();
    let value = parse(inner.next().unwrap());

    (key, value)
}

fn make_param_macro(pair: Pair<Rule>) -> (String, Value) {
    let mut inner = pair.into_inner();
    let mut map = HashMap::new();
    let key = inner.next().unwrap().as_str().to_string();
    let attr2 = inner.next().unwrap();
    println!("attr2 {:?}", attr2);

    match attr2.as_rule() {
        Rule::param_macro_content => {
            let params = parse(attr2);
            (key, params)
        }
        _ => {
            let value = parse(attr2);
            map.insert(key.clone(), value);
            let params = parse(inner.next().unwrap());
            (key, params.merge_object(map).unwrap())
        }
    }
}

fn parse(pair: Pair<Rule>) -> Value {
    match pair.as_rule() {
        Rule::sessions => {
            let mut item: HashMap<String, Value> = HashMap::new();

            for pair in pair.into_inner() {
                match pair.as_rule() {
                    Rule::session_generic_config => {
                        let mut inner = pair.into_inner();
                        let name = inner.next().unwrap().as_str().to_string();
                        let value = parse(inner.next().unwrap());

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

                        let value = parse(inner.next().unwrap());

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
            parse(pair.into_inner().next().unwrap())
        }
        Rule::session_pipeline_content => {
            let mut list = Vec::new();

            for pair in pair.into_inner() {
                list.push(parse(pair));
            }

            Value::Array(list)
        }
        Rule::module => {
            let mut inner = pair.into_inner();
            let module_name = Value::String(inner.next().unwrap().as_str().to_string());
            let mut map = HashMap::new();
            let params = parse(inner.next().unwrap());

            map.insert("module".to_string(), module_name);
            map.extend(params.to_object().unwrap());

            Value::Object(map)
        }
        Rule::module_content => {
            let mut map = HashMap::new();

            for pair in pair.into_inner() {
                let rule = pair.as_rule();
                let value = parse(pair);

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
            let value = parse(inner.next().unwrap());

            value
        }
        Rule::param_macro_content => {
            // common_content
            match pair.into_inner().next() {
                Some(pair) => parse(pair),
                None => Value::Object(map!()),
            }
        }
        Rule::common_content => {
            let mut map = HashMap::new();

            for pair in pair.into_inner() {
                match pair.as_rule() {
                    Rule::param => {
                        let value = make_param(pair);
                        map.insert(value.0, value.1);
                    }
                    Rule::param_macro => {
                        let (name, value) = make_param_macro(pair);

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
            let mut map = HashMap::new();
            for pair in pair.into_inner() {
                let mut inner = pair.into_inner();
                let key = inner.next().unwrap().as_str().to_string();
                let value = parse(inner.next().unwrap());

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
            parse(inner.next().unwrap())
        }
        Rule::string_content => Value::String(pair.as_str().to_string()),
        Rule::ident => Value::String(pair.as_str().to_string()),
        Rule::string_interpolation => {
            let mut inner = pair.clone().into_inner();
            parse(inner.next().unwrap())
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
        _ => {
            println!("Exception {:?}", pair.as_rule());
            println!("Exception Content {:?}", pair.as_span());
            Value::Undefined
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        pipe();
        assert!(true);
    }
}
