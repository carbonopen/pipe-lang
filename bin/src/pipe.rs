use std::{collections::HashMap, convert::TryFrom};

use pipe_parser::value::Value;
use serde_json::Value as JsonValue;

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Step {
    pub module: String,
    pub params: Option<JsonValue>,
    pub reference: Option<String>,
    pub producer: Option<bool>,
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
}

impl TryFrom<&Value> for Module {
    type Error = ();

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let obj = value.to_object().unwrap();
        let bin = obj.get("bin").unwrap();

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
                obj.get("name").unwrap().to_string().unwrap(),
            )
        } else {
            return Err(());
        };

        Ok(Self { name, bin })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Pipe {
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

        println!("{:?}", pipeline);

        for item in pipeline {
            let obj = item.to_object().unwrap();
            let module = obj.get("module").unwrap().to_string().unwrap();
            let reference = match obj.get("ref").unwrap().to_string() {
                Ok(value) => Some(value),
                Err(_) => None,
            };
            let params = if let Some(params) = obj.get("params") {
                Some(params.as_json())
            } else {
                None
            };
            // let producer = obj.get("producer").unwrap().to_boolean().unwrap_or(false);

            // list.push(Step {
            //     module,
            //     params,
            //     reference: todo!(),
            //     producer: todo!(),
            //     attach: todo!(),
            // });
        }

        list
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

        Ok(Self {
            modules,
            pipeline,
            vars,
        })
    }
}
