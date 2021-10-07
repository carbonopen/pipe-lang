extern crate pest;
#[macro_use]
extern crate pest_derive;
mod pipe;
mod value;

use pest::iterators::Pairs;
use pest::Parser;
use pest::{error::Error, iterators::Pair};
use pipe::Pipe;
use std::fs;
#[derive(Parser)]
#[grammar = "pipe.pest"]
pub struct PipeParser;

fn make() {
    let unparsed_file = fs::read_to_string("../demo/example.pipe").expect("cannot read file");

    let pairs = PipeParser::parse(Rule::pipe, &unparsed_file).unwrap();

    let json = parse_pipe(pairs);

    println!("{:#?}", &json);
}

fn parse_pipe(pairs: Pairs<Rule>) -> Pipe {
    let mut result = Pipe::default();

    for pair in pairs {
        match pair.as_rule() {
            Rule::sessions => println!("Digit:   {}", pair.as_str()),
            _ => unreachable!(),
        };
    }

    result
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
