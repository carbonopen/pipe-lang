pub mod step;
use crate::extensions::Extension;
use crate::extensions::ExtensionType;
use libloading::{Library, Symbol};
use pipe_parser::value::Value;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs;
use step::Step;

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
    pub fn new(value: &Value, runtime_extension_path: &str) -> Self {
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

            if cfg!(feature = "extensions") {
                Self::pipeline_to_steps(&obj, args, runtime_extension_path)
            } else {
                Self::to_steps(&obj, args)
            }
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

    #[cfg(feature = "extensions")]
    fn pipeline_to_steps(
        pipeline: &Vec<Value>,
        args: Option<Value>,
        runtime_extension_path: &str,
    ) -> Vec<Step> {
        let entries = fs::read_dir(runtime_extension_path).unwrap();
        let mut steps = Self::to_steps(pipeline, args);

        for entry in entries {
            if let Ok(entry) = entry {
                match entry.file_type() {
                    Ok(file_type) if file_type.is_file() => {
                        let path = entry.path().display().to_string();

                        if path.ends_with(".lrepos") {
                            let lib = match Library::new(path.clone()) {
                                Ok(lib) => lib,
                                Err(err) => panic!("Error: {}; Filename: {}", err, path),
                            };

                            let bin = unsafe {
                                let constructor: Symbol<
                                    unsafe extern "C" fn() -> *mut dyn Extension,
                                > = lib.get(b"_Extension").unwrap();
                                let boxed_raw = constructor();
                                Box::from_raw(boxed_raw)
                            };

                            if bin.extension_type().eq(&ExtensionType::PosParse) {
                                bin.handler(&mut steps)
                            }
                        }
                    }
                    Ok(_) => continue,
                    Err(err) => panic!("{:?}", err),
                }
            }
        }

        steps
    }
}
