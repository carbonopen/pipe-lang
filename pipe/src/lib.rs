extern crate pest;
#[macro_use]
extern crate pest_derive;
mod value;

use pest::iterators::Pair;
use pest::Parser;
use std::collections::HashMap;
use std::fs;
use value::Value;
#[derive(Parser)]
#[grammar = "pipe.pest"]
pub struct PipeParser;

fn pipe() {
    let unparsed_file = fs::read_to_string("../demo/example.pipe").expect("cannot read file");

    let mut pair = PipeParser::parse(Rule::pipe, &unparsed_file)
        .unwrap()
        .next()
        .unwrap();

    let json = parse(pair);

    println!("");
    println!("RESULT: {:#?}", json);
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
    let value = parse(inner.next().unwrap());
    map.insert(key.clone(), value);

    let params = parse(inner.next().unwrap());

    (key, params.merge_object(map).unwrap())
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
                    Rule::session_pipeline => {}
                    _ => {}
                }
            }

            Value::Object(item)
        }
        Rule::session_generic_config_content => {
            // common_content
            parse(pair.into_inner().next().unwrap())
        }
        Rule::param_macro_content => {
            // common_content
            parse(pair.into_inner().next().unwrap())
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
        Rule::string => Value::set_string_from_string(pair.as_str()),
        Rule::ident => Value::String(pair.as_str().to_string()),
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
