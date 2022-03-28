use crate::pipe::Step;

mod sort;

pub use sort::Sort;

pub trait PreParse {
    fn parse(list: Vec<Step>) -> Vec<Step> {
        list
    }
}
