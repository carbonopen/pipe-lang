use std::collections::HashMap;

use serde_json::Value as JsonValue;

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Step {
    pub id: usize,
    pub position: usize,
    pub module: String,
    pub params: Option<JsonValue>,
    pub reference: Option<String>,
    pub tags: HashMap<String, JsonValue>,
    pub attach: Option<String>,
    pub args: HashMap<String, JsonValue>,
}