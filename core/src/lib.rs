extern crate pest;
#[macro_use]
extern crate pest_derive;
mod r#struct;
mod value;

use pest::Parser;
use pest::{error::Error, iterators::Pair};
use std::fs;

use crate::value::Value;
#[derive(Parser)]
#[grammar = "co2.pest"]
pub struct Co2Parser;

fn co2() {
    let unparsed_file = fs::read_to_string("../demo/example.co2").expect("cannot read file");

    let json = Co2Parser::parse(Rule::co2, &unparsed_file)
        .unwrap()
        .next()
        .unwrap();
    // println!("{:#?}", &json);
    // Ok(parse_value(json))
    println!("{:#?}", &json);
}

// fn parse_value(pair: Pair<Rule>) -> Value {
//     match pair.as_rule() {
//         Rule::object => Value::Object(
//             pair.into_inner()
//                 .map(|pair| {
//                     let mut inner_rules = pair.into_inner();
//                     let name = inner_rules
//                         .next()
//                         .unwrap()
//                         .into_inner()
//                         .next()
//                         .unwrap()
//                         .as_str();
//                     let value = parse_value(inner_rules.next().unwrap());
//                     (name, value)
//                 })
//                 .collect(),
//         ),
//         Rule::array => Value::Array(pair.into_inner().map(parse_value).collect()),
//         Rule::string => Value::String(pair.into_inner().next().unwrap().as_str()),
//         Rule::number => Value::Number(pair.as_str().parse().unwrap()),
//         Rule::boolean => Value::Boolean(pair.as_str().parse().unwrap()),
//         Rule::null => Value::Null,
//         Rule::co2
//         | Rule::EOI
//         | Rule::pair
//         | Rule::value
//         | Rule::inner
//         | Rule::char
//         | Rule::WHITESPACE => unreachable!(),
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {
        co2();
    }
}
