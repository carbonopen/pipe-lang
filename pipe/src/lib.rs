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

fn make() {
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
    let mut obj = params.to_object().unwrap();
    obj.extend(map);

    (key, Value::Object(obj))
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

                        match item.get_mut(&name) {
                            Some(cur_value) => {
                                // let mut map = HashMap::new();
                                // map.insert(name, value);
                                // let cur = cur_value;
                                // cur.merge_object(map);
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
            let mut item = HashMap::new();

            for pair in pair.into_inner() {
                let value = match pair.as_rule() {
                    Rule::param => make_param(pair),
                    Rule::param_macro => make_param_macro(pair),
                    _ => ("".to_string(), Value::Null),
                };

                item.insert(value.0, value.1);
            }

            Value::Object(item)
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
            Value::Null
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        make();
        assert!(true);
    }
}
