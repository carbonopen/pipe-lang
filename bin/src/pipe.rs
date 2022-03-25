use pipe_parser::value::Value;
use serde_json::Value as JsonValue;
use std::{collections::HashMap, convert::TryFrom};

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Step {
    pub module: String,
    pub params: Option<JsonValue>,
    pub reference: Option<String>,
    pub tags: HashMap<String, JsonValue>,
    pub attach: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Payload {
    pub request: Option<JsonValue>,
    pub response: Option<JsonValue>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Module {
    pub name: String,
    pub bin: String,
    pub params: HashMap<String, JsonValue>,
}

impl TryFrom<&Value> for Module {
    type Error = ();

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let params = value.to_object().unwrap();
        let bin = params.get("bin").unwrap();

        let (bin, name) = if bin.is_array() {
            let array = bin.to_array().unwrap();
            let mut array_item = array.iter();

            (
                array_item.next().unwrap().to_string().unwrap(),
                array_item.next().unwrap().to_string().unwrap(),
            )
        } else if bin.is_string() {
            (
                bin.to_string().unwrap(),
                params.get("name").unwrap().to_string().unwrap(),
            )
        } else {
            return Err(());
        };

        let val_json = value.as_json();
        let params = serde_json::from_str(&val_json).unwrap();

        Ok(Self { name, bin, params })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Pipe {
    pub config: Option<HashMap<String, Value>>,
    pub vars: Option<HashMap<String, Value>>,
    pub modules: Option<Vec<Module>>,
    pub pipeline: Vec<Step>,
}

impl Pipe {
    fn load_modules(imports: &HashMap<String, Value>) -> Vec<Module> {
        let mut modules = Vec::new();

        for (import_type, value) in imports {
            if import_type.eq("bin") {
                value
                    .to_array()
                    .unwrap()
                    .iter()
                    .for_each(|item| match Module::try_from(item) {
                        Ok(module) => modules.push(module),
                        Err(_) => (),
                    });
            }
        }

        modules
    }

    fn pipeline_to_steps(pipeline: &Vec<Value>) -> Vec<Step> {
        let mut list = Vec::new();
        let mut references = HashMap::new();

        for item in pipeline {
            let obj = item.to_object().unwrap();
            let module = obj.get("module").unwrap().to_string().unwrap();
            let reference = match obj.get("ref").unwrap().to_string() {
                Ok(value) => Some(value),
                Err(_) => None,
            };
            let (params, attach) = if let Some(params) = obj.get("params") {
                let mut obj = params.to_object().unwrap();
                let attach = if let Some(attach) = obj.get("attach") {
                    Some(attach.to_string().unwrap())
                } else {
                    None
                };

                if attach.is_some() {
                    obj.remove("attach");
                }

                let val = Value::Object(obj);
                let val_json = val.as_json();

                (Some(serde_json::from_str(&val_json).unwrap()), attach)
            } else {
                (None, None)
            };

            let tags = match obj.get("tags") {
                Some(value) => {
                    let json = value.as_json();
                    serde_json::from_str(&json).unwrap()
                }
                None => HashMap::default(),
            };

            if let Some(reference) = reference.clone() {
                references.insert(reference, list.len());
            }

            list.push(Step {
                module,
                params,
                reference,
                tags,
                attach,
            });
        }

        let mut order = list.clone();

        for (index, item) in list.iter().enumerate() {
            let step = match item.tags.get("step") {
                Some(value) => value.as_i64().unwrap(),
                None => -1,
            };

            if item.tags.get("producer").is_some() || item.tags.get("first").is_some() {
                order.remove(index);
                order.insert(0, item.clone());
            } else if item.tags.get("last").is_some() {
                order.remove(index);
                order.push(item.clone());
            } else if step.ge(&0) {
                order.remove(index);
                order.insert(step as usize, item.clone());
            } else if let Some(value) = item.tags.get("before") {
                let refer = match value.as_array() {
                    Some(refer) => Some(refer.get(0).unwrap().as_str().unwrap().to_string()),
                    None => continue,
                };

                let step = match order.iter().position(|item| item.reference.eq(&refer)) {
                    Some(step) => {
                        let res = step - 1;
                        if res.lt(&0) {
                            0
                        } else {
                            res
                        }
                    }
                    None => panic!("Unable to order modules by reference {}.", refer.unwrap()),
                };

                order.remove(index);
                order.insert(step, item.clone());
            } else if let Some(value) = item.tags.get("after") {
                let refer = match value.as_array() {
                    Some(refer) => Some(refer.get(0).unwrap().as_str().unwrap().to_string()),
                    None => continue,
                };

                let step = match order.iter().position(|item| item.reference.eq(&refer)) {
                    Some(step) => step,
                    None => panic!("Unable to order modules by reference {}.", refer.unwrap()),
                };

                order.remove(index);
                order.insert(step, item.clone());
            }
        }

        order
    }
}

impl TryFrom<&Value> for Pipe {
    type Error = ();

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let pipe_obj = value.to_object().expect("Error trying to capture code.");
        let modules = match pipe_obj.get("import") {
            Some(value) => match value.to_object() {
                Ok(obj) => Some(Self::load_modules(&obj)),
                Err(_) => None,
            },
            None => None,
        };
        let pipeline = {
            let pipeline = pipe_obj.get("pipeline").expect("No pipeline present.");
            let obj = pipeline.to_array().expect("Could not load pipeline");
            Self::pipeline_to_steps(&obj)
        };
        let vars = Default::default();
        let config = Default::default();

        Ok(Self {
            config,
            modules,
            pipeline,
            vars,
        })
    }
}
