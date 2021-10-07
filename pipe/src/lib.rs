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

    let mut pair = PipeParser::parse(Rule::pipe, &unparsed_file)
        .unwrap()
        .next()
        .unwrap();

    let json = parse_pipe::<Pipe>(pair);

    // println!("{:#?}", json);
}

fn parse_pipe<T>(pair: Pair<Rule>) -> T
where
    T: Default,
{
    match pair.as_rule() {
        Rule::sessions => {
            // let mut items = Vec::new();
            // println!("Digit:   {}", pair.clone().into_inner());

            let item = pair
                .into_inner()
                .map(|pair| {
                    println!("Digit:   {}", pair.clone().into_inner());
                    
                })
                .collect::<Vec<_>>();

            println!("Digit: {:?}", item);

            T::default()
        }
        _ => unreachable!(),
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
