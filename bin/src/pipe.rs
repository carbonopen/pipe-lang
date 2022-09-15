use crate::pos_parse;
use crate::pos_parse::PosParse;
use pipe_parser::value::Value;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

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

#[derive(Debug, PartialEq, Clone)]
pub struct Payload {
    pub request: Option<JsonValue>,
    pub response: Option<JsonValue>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ModuleType {
    Bin,
    Pipeline,
}

impl ModuleType {
    pub fn get_name<'a>(&self) -> &'a str {
        match self {
            ModuleType::Bin => "bin",
            ModuleType::Pipeline => "mod",
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Module {
    pub name: String,
    pub path: String,
    pub params: HashMap<String, JsonValue>,
    pub module_type: ModuleType,
}

impl Module {
    fn new(value: &Value, module_type: ModuleType) -> Self {
        let mut params = value.to_object().unwrap();
        let mut path = params
            .remove(module_type.get_name())
            .unwrap()
            .to_string()
            .unwrap();

        if module_type.eq(&ModuleType::Pipeline) {
            path += ".pipe";
        }

        params.insert("path".to_string(), Value::String(path.clone()));

        let name = match params.get("name") {
            Some(name) => name.to_string().unwrap(),
            None => {
                let name = path
                    .split("/")
                    .last()
                    .unwrap()
                    .split(".")
                    .next()
                    .unwrap()
                    .to_string();
                params.insert("name".to_string(), Value::String(name.clone()));
                name
            }
        };

        let val_json = value.as_json();
        let params = serde_json::from_str(&val_json).unwrap();

        Self {
            name,
            path,
            params,
            module_type,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Pipe {
    pub config: Option<HashMap<String, Value>>,
    pub args: Option<HashMap<String, Value>>,
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
                    .for_each(|item| modules.push(Module::new(item, ModuleType::Bin)));
            } else if import_type.eq("mod") {
                value
                    .to_array()
                    .unwrap()
                    .iter()
                    .for_each(|item| modules.push(Module::new(item, ModuleType::Pipeline)));
            }
        }

        modules
    }

    fn to_steps(pipeline: &Vec<Value>, args: Option<Value>) -> Vec<Step> {
        let mut list = Vec::new();
        let mut references = HashMap::new();
        let args = match args {
            Some(value) => {
                let args = value.to_object().unwrap();
                let mut map = HashMap::new();

                for (key, value) in args {
                    match value {
                        Value::Object(value) => match value.get("___PIPE___type") {
                            Some(var_type) => match var_type.to_string() {
                                Ok(var_type) => {
                                    if var_type.eq("converter") {
                                        let var_type = value
                                            .get("___PIPE___value")
                                            .unwrap()
                                            .to_string()
                                            .unwrap();

                                        let var_value = match value.get("___PIPE___default") {
                                            Some(var_default) => var_default.clone(),
                                            None => {
                                                if var_type.eq("String") {
                                                    Value::String(String::default())
                                                } else if var_type.eq("Number") {
                                                    Value::Number("0".to_string())
                                                } else if var_type.eq("Boolean") {
                                                    Value::Boolean(false)
                                                } else if var_type.eq("Array") {
                                                    Value::Array(vec![Value::Empty])
                                                } else if var_type.eq("Object") {
                                                    Value::Object(HashMap::default())
                                                } else {
                                                    Value::Undefined
                                                }
                                            }
                                        };

                                        map.insert(key, var_value);
                                    } else {
                                        map.insert(key, Value::Object(value));
                                    }
                                }
                                Err(_) => panic!("Could not convert ___PIPE___type."),
                            },
                            None => {
                                map.insert(key, Value::Object(value));
                            }
                        },
                        _ => {
                            map.insert(key, value);
                        }
                    };
                }

                let json = Value::Object(map).as_json();
                serde_json::from_str(&json).unwrap()
            }
            None => HashMap::default(),
        };

        for (id, item) in pipeline.iter().enumerate() {
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
                id,
                position: id,
                module,
                params,
                reference,
                tags,
                attach,
                args: args.clone(),
            });
        }

        list
    }

    fn pipeline_to_steps(pipeline: &Vec<Value>, args: Option<Value>) -> Vec<Step> {
        let list = Self::to_steps(pipeline, args);
        let sort = pos_parse::Sort::parse(list);

        sort
    }
}

impl Pipe {
    pub fn new(value: &Value) -> Self {
        let pipe_obj = value.to_object().expect("Error trying to capture code.");
        let modules = match pipe_obj.get("import") {
            Some(value) => match value.to_object() {
                Ok(obj) => Some(Self::load_modules(&obj)),
                Err(_) => None,
            },
            None => None,
        };
        let pipeline = {
            let args = match pipe_obj.get("args") {
                Some(args) => Some(args.clone()),
                None => None,
            };
            let pipeline = pipe_obj.get("pipeline").expect("No pipeline present.");
            let obj = pipeline.to_array().expect("Could not load pipeline");
            Self::pipeline_to_steps(&obj, args)
        };

        let args = Default::default();
        let config = Default::default();

        Self {
            config,
            modules,
            pipeline,
            args,
        }
    }
}


