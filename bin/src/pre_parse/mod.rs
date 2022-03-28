use crate::pipe::Step;

pub mod sort;

pub trait PreParse {
    fn parse(list: Vec<Step>) -> Vec<Step> {
        list
    }
}
