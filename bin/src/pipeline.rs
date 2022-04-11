use libloading::{Library, Symbol};
use pipe_core::{
    debug, log,
    modules::{BinSender, Config, History, Module, Request, Response, ID},
};
use serde_json::{Map, Value};

use std::sync::mpsc::{Receiver, Sender};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use std::{sync::mpsc, thread};

use crate::{
    pipe::{ModuleType, Pipe},
    runtime::{Modules, PipelineConfig, PipelineResponse, PipelineSetup},
};

#[derive(Default, Clone)]
struct Reference {
    name_to_id: HashMap<String, u32>,
    id_to_name: HashMap<u32, String>,
}

impl Reference {
    pub fn add(&mut self, name: String, id: u32) {
        self.name_to_id.insert(name.clone(), id);
        self.id_to_name.insert(id, name);
    }

    pub fn get_by_name(&self, name: &str) -> u32 {
        self.name_to_id.get(name).unwrap().clone()
    }

    pub fn get_by_id(&self, id: u32) -> String {
        self.id_to_name.get(&id).unwrap().clone()
    }
}

#[derive(Debug)]
pub struct StepConfig {
    pub id: u32,
    pub reference: String,
    pub params: Map<String, Value>,
    pub producer: bool,
    pub default_attach: Option<String>,
    pub tags: HashMap<String, Value>,
    pub args: HashMap<String, Value>,
}

#[derive(Debug)]
struct Step {
    pub module_type: ModuleType,
    pub sender: Option<Sender<Request>>,
    pub config: StepConfig,
}

impl Step {
    pub fn send(&self, request: Request) -> Result<(), ()> {
        match self.module_type {
            ModuleType::Bin => match &self.sender {
                Some(sender) => match sender.send(request) {
                    Ok(_) => Ok(()),
                    Err(_) => Err(()),
                },
                None => Err(()),
            },
            ModuleType::Pipeline => Err(()),
        }
    }
}

#[derive(Debug)]
struct Handler {
    pub steps: HashMap<u32, Step>,
    pub history: Arc<Mutex<History>>,
    total_bins: u32,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            steps: HashMap::default(),
            history: Arc::new(Mutex::new(History::new())),
            total_bins: 0,
        }
    }

    pub fn insert_pipeline(&mut self, id: u32, config: StepConfig) {
        self.steps.insert(
            id,
            Step {
                module_type: ModuleType::Pipeline,
                sender: None,
                config,
            },
        );
    }

    pub fn insert_bin(&mut self, id: u32, config: StepConfig) {
        self.steps.insert(
            id,
            Step {
                module_type: ModuleType::Bin,
                sender: None,
                config,
            },
        );
        self.total_bins += 1;
    }

    pub fn bin_sender(&mut self, id: u32, sender: Sender<Request>) {
        match self.steps.get_mut(&id) {
            Some(step) => step.sender = Some(sender),
            None => (),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub id: u32,
    pub key: String,
    pipe: Pipe,
}

impl Pipeline {
    pub fn new(id: u32, key: String, pipe: Pipe) -> Self {
        Self { id, key, pipe }
    }

    pub fn start(
        &self,
        modules: Modules,
        sender_setup_runtime: Sender<PipelineSetup>,
        sender_response_runtime: Sender<PipelineResponse>,
    ) -> Result<(), ()> {
        let (sender_request_runtime, receiver_response_runtime): (
            Sender<Request>,
            Receiver<Request>,
        ) = mpsc::channel();

        if sender_setup_runtime
            .send(PipelineSetup {
                tx: sender_request_runtime.clone(),
                id: self.id,
            })
            .is_err()
        {
            return Err(()); // TODO: definir
        }

        let (tx_senders, rx_senders): (Sender<BinSender>, Receiver<BinSender>) = mpsc::channel();
        let (tx_control, rx_control): (Sender<Response>, Receiver<Response>) = mpsc::channel();
        let mut module_id: ID = 0;
        let mut modules_reference = Reference::default();

        let module_by_name = match self.pipe.modules.clone() {
            Some(modules) => {
                let mut result = HashMap::new();

                for module in modules.iter() {
                    result.insert(module.name.clone(), module.clone());
                }

                result
            }
            None => HashMap::default(),
        };

        let mut handler = Handler::new();

        for step in self.pipe.pipeline.iter() {
            let step = step.clone();

            let current_module = match module_by_name.get(&step.module) {
                Some(a) => a,
                None => {
                    log::error!(r#"Module ¨"{}" not load in "{}""#, step.module, self.key);
                    continue;
                }
            };

            let reference = match step.reference {
                Some(reference) => reference,
                None => format!("step-{}", &module_id),
            };

            modules_reference.add(reference.clone(), module_id.clone());

            let mut params = match step.params {
                Some(params) => match params.as_object() {
                    Some(params) => params.clone(),
                    None => Map::new(),
                },
                None => Map::new(),
            };
            let producer = step.tags.get("producer").is_some();
            let default_attach = step.attach;
            let mut module_setup_params = current_module.params.clone();
            let module_inner = modules.get(&self.key, &current_module.name);
            let id = module_id.clone();
            let tags = step.tags.clone();
            let args = step.args.clone();
            let reference = reference.clone();

            module_id = module_id + 1;

            if module_inner.module_type.eq(&ModuleType::Pipeline) {
                module_setup_params.remove("name");
                module_setup_params.remove("mod");

                for (key, value) in module_setup_params {
                    params.insert(key, value);
                }

                handler.insert_pipeline(
                    id,
                    StepConfig {
                        id,
                        reference,
                        params,
                        producer,
                        default_attach,
                        tags,
                        args,
                    },
                );
            } else if module_inner.module_type.eq(&ModuleType::Bin) {
                module_setup_params.remove("name");
                module_setup_params.remove("bin");

                for (key, value) in module_setup_params {
                    params.insert(key, value);
                }

                handler.insert_bin(
                    id,
                    StepConfig {
                        id,
                        reference: reference.clone(),
                        params: params.clone(),
                        producer,
                        default_attach: default_attach.clone(),
                        tags: tags.clone(),
                        args: args.clone(),
                    },
                );

                let response = tx_control.clone();
                let request = tx_senders.clone();
                let bin_key = modules.get_bin_key(&module_inner.name);
                let config = Config {
                    reference,
                    params,
                    producer,
                    default_attach,
                    tags,
                    args,
                };

                thread::spawn(move || {
                    let lib = match Library::new(bin_key.clone()) {
                        Ok(lib) => lib,
                        Err(err) => panic!("Error: {}; Filename: {}", err, bin_key),
                    };
                    let bin = unsafe {
                        let constructor: Symbol<unsafe extern "C" fn() -> *mut dyn Module> =
                            lib.get(b"_Module").unwrap();
                        let boxed_raw = constructor();
                        Box::from_raw(boxed_raw)
                    };

                    bin.start(id, request, response, config);
                });
            }
        }

        let mut limit_senders = handler.total_bins - 1;
        for sender in rx_senders {
            handler.bin_sender(sender.id, sender.tx);

            if limit_senders == 0 {
                break;
            }

            limit_senders -= 1;
        }

        for control in rx_control {
            log::trace!(
                "trace_id: {} | Step {} sender: {:?}",
                control.trace_id,
                control.origin,
                control
            );

            let module_name = modules_reference.get_by_id(control.origin);
            let mut his_lock = handler.history.lock().unwrap();

            his_lock.insert(control.trace_id, module_name.clone(), control.clone());

            let steps = match his_lock.steps.get(&control.trace_id) {
                Some(steps) => Some(steps.clone()),
                None => None,
            };

            let request = Request {
                origin: control.origin,
                payload: control.payload,
                trace_id: control.trace_id,
                steps,
            };

            if let Some(attach) = control.attach {
                log::trace!(
                    "trace_id: {} | Resolving attach: {}",
                    control.trace_id,
                    attach.clone()
                );

                let step = handler
                    .steps
                    .get(&modules_reference.get_by_name(&attach))
                    .unwrap();

                match step.send(request) {
                    Ok(_) => continue,
                    Err(err) => {
                        log::error!("{:#?}", err);
                    }
                };
            } else {
                let next_step = control.origin + 1;

                match handler.steps.get(&next_step) {
                    Some(step) => match step.send(request) {
                        Ok(_) => continue,
                        Err(err) => {
                            log::error!("{:#?}", err);
                        }
                    },
                    None if control.origin > 0 => {
                        log::trace!(
                            "trace_id: {} |  Step id {} not exist, send to step id 0",
                            control.trace_id,
                            next_step
                        );

                        match &handler.steps.get(&0) {
                            Some(step) => match step.send(request) {
                                Ok(_) => continue,
                                Err(err) => {
                                    log::error!("{:#?}", err);
                                }
                            },
                            None => {
                                log::warn!(
                                    "trace_id: {} |  Sender by step id {} not exist",
                                    control.trace_id,
                                    next_step
                                );
                            }
                        };
                    }
                    None => (),
                };
            }
        }

        Ok(())
    }
}
