use std::collections::HashMap;

// use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug)]
pub struct Number {
    n: String,
}

pub struct Interpolation {
    n: String,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Object(Vec<(String, Value)>),
    Array(Vec<Value>),
    String(String),
    Number(f64),
    Boolean(bool),
    Interpolation(String),
    Null,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Pipe {
    pub pipeline: Vec<Module>,
    pub config: Config,
    pub vars: Vars,
    pub import: Import,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Module {
    pub module: String,
    pub name: String,
    pub params: HashMap<String, Value>,
    pub attach: String,
}

type Config = HashMap<String, Value>;
type Vars = HashMap<String, Value>;
type Import = HashMap<String, Value>;
type Pipeline = Vec<Module>;
