use pipe_parser::value::Value;
use serde_json::Value as JsonValue;
use std::{collections::HashMap, convert::TryFrom};

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Step {
    pub id: usize,
    pub position: usize,
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

#[derive(Debug, PartialEq, Eq)]
enum Direction {
    Forward,
    Backward,
    None,
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

    fn change_position(
        mut map: HashMap<usize, Step>,
        index: usize,
        from: usize,
        direction: Direction,
    ) -> HashMap<usize, Step> {
        if index == from {
            map
        } else {
            let content = map.get(&index).unwrap().clone();
            map.remove(&index);

            if map.get(&from).is_none() {
                map.insert(from, content);
                map
            } else {
                let mut map = {
                    if direction.eq(&Direction::None) {
                        if index < from {
                            Self::change_position(map, from, from - 1, Direction::Backward)
                        } else {
                            Self::change_position(map, from, from + 1, Direction::Forward)
                        }
                    } else if direction.eq(&Direction::Forward) {
                        Self::change_position(map, from, from + 1, Direction::Forward)
                    } else {
                        Self::change_position(map, from, from - 1, Direction::Backward)
                    }
                };
                map.insert(from, content);
                map
            }
        }
    }

    fn pipeline_to_steps(pipeline: &Vec<Value>) -> Vec<Step> {
        let mut list = Vec::new();
        let mut references = HashMap::new();

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
            });
        }

        let mut order = HashMap::new();

        for (index, item) in list.iter().enumerate() {
            order.insert(index, item.clone());
        }

        let mut step_first = Vec::new();
        let mut step_last = Vec::new();
        let mut by_reference_before = Vec::new();
        let mut by_reference_after = Vec::new();

        for item in list.iter() {
            if let Some(value) = item.tags.get("step") {
                match value.as_array() {
                    Some(value) => {
                        let val = value.get(0).unwrap();
                        let step = val.as_i64().unwrap() as usize;

                        order = Self::change_position(order, item.id, step, Direction::None);
                    }
                    None => panic!("Unable to order modules by step {}.", item.id),
                }
            } else if item.tags.get("first").is_some() {
                step_first.push(item);
            } else if item.tags.get("last").is_some() {
                step_last.push(item);
            } else if let Some(value) = item.tags.get("before") {
                let refer = match value.as_array() {
                    Some(refer) => Some(refer.get(0).unwrap().as_str().unwrap().to_string()),
                    None => continue,
                };

                by_reference_before.push((refer, item.clone()));
            } else if let Some(value) = item.tags.get("after") {
                let refer = match value.as_array() {
                    Some(refer) => Some(refer.get(0).unwrap().as_str().unwrap().to_string()),
                    None => continue,
                };

                by_reference_after.push((refer, item.clone()));
            }
        }

        // First
        for step in step_first {
            let first = order
                .iter()
                .find_map(|(i, a)| if a.id.eq(&step.id) { Some(i) } else { None })
                .unwrap()
                .clone();

            order = Self::change_position(order, first, 0, Direction::None);
        }

        // After
        for (refer, step) in by_reference_after {
            let refer_index = match order.iter().find_map(|(i, a)| {
                if a.reference.eq(&refer) {
                    Some(i)
                } else {
                    None
                }
            }) {
                Some(index) => *index + 1,
                None => {
                    panic!("Referencia não encontrada: {}", refer.unwrap());
                }
            };

            let target_index = order
                .iter()
                .find_map(|(i, a)| if a.id.eq(&step.id) { Some(i) } else { None })
                .unwrap()
                .clone();

            order = Self::change_position(order, target_index, refer_index, Direction::None);
        }

        // Last
        let last_index = list.len() - 1;

        for step in step_last {
            let last = order
                .iter()
                .find_map(|(i, a)| if a.id.eq(&step.id) { Some(i) } else { None })
                .unwrap()
                .clone();

            order = Self::change_position(order, last, last_index, Direction::None);
        }

        // Before
        for (refer, step) in by_reference_before {
            let refer_index = match order.iter().find_map(|(i, a)| {
                if a.reference.eq(&refer) {
                    Some(i)
                } else {
                    None
                }
            }) {
                Some(index) => *index - 1,
                None => {
                    panic!("Referencia não encontrada: {}", refer.unwrap());
                }
            };

            let target_index = order
                .iter()
                .find_map(|(i, a)| if a.id.eq(&step.id) { Some(i) } else { None })
                .unwrap()
                .clone();

            order = Self::change_position(order, target_index, refer_index, Direction::None);
        }
        let mut order_list: Vec<_> = order.into_iter().collect();

        order_list.sort_by(|x, y| x.0.cmp(&y.0));

        let order_list = order_list.iter().map(|a| a.1.clone()).collect::<Vec<_>>();

        order_list
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
