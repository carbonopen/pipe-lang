use crate::pipe::Step;

mod sort;

pub use sort::Sort;

pub trait PosParse {
    fn parse(list: Vec<Step>) -> Vec<Step> {
        list
    }
}
