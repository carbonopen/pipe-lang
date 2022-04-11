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
    runtime::{Alias, Aliases, Modules, PipelineRequest, PipelineSetup},
};

#[derive(Debug, Clone)]
pub struct StepConfig {
    pub id: u32,
    pub reference: String,
    pub params: Map<String, Value>,
    pub producer: bool,
    pub default_attach: Option<String>,
    pub tags: HashMap<String, Value>,
    pub args: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
struct Step {
    pub id: u32,
    pub key: String,
    pub module_type: ModuleType,
    pub sender: Option<Sender<Request>>,
    pub config: StepConfig,
    pub sender_pipeline: Option<Sender<PipelineRequest>>,
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
            ModuleType::Pipeline => match &self.sender_pipeline {
                Some(sender) => {
                    match sender.send(PipelineRequest::from_request(
                        self.id,
                        self.key.clone(),
                        request,
                    )) {
                        Ok(_) => Ok(()),
                        Err(_) => Err(()),
                    }
                }
                None => Err(()),
            },
        }
    }
}

#[derive(Debug, Clone)]
struct Handler {
    key: String,
    pub steps: HashMap<u32, Step>,
    pub history: Arc<Mutex<History>>,
    pub reference: HashMap<String, u32>,
    total_bins: u32,
    pipeline_sender: Sender<PipelineRequest>,
    alias: Alias,
}

impl Handler {
    pub fn new(pipeline_sender: Sender<PipelineRequest>, alias: Alias, key: String) -> Self {
        Self {
            key,
            steps: HashMap::default(),
            history: Arc::new(Mutex::new(History::new())),
            total_bins: 0,
            reference: HashMap::default(),
            pipeline_sender,
            alias,
        }
    }

    pub fn insert_pipeline(&mut self, id: u32, alias: &str, config: StepConfig) {
        self.reference.insert(config.reference.clone(), id);

        let key = self.alias.get(alias).unwrap().key.clone();

        self.steps.insert(
            id,
            Step {
                id,
                key,
                module_type: ModuleType::Pipeline,
                sender: None,
                config,
                sender_pipeline: Some(self.pipeline_sender.clone()),
            },
        );
    }

    pub fn insert_bin(&mut self, id: u32, alias: &str, config: StepConfig) {
        self.reference.insert(config.reference.clone(), id);

        let key = self.alias.get(alias).unwrap().key.clone();

        self.steps.insert(
            id,
            Step {
                id,
                key,
                module_type: ModuleType::Bin,
                sender: None,
                config,
                sender_pipeline: None,
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

    pub fn update_history(
        &self,
        response: &Response,
    ) -> Option<HashMap<String, pipe_core::modules::Step>> {
        match self.steps.get(&response.origin) {
            Some(step) => {
                let mut his_lock = self.history.lock().unwrap();

                his_lock.insert(
                    response.trace_id,
                    step.config.reference.clone(),
                    response.clone(),
                );

                match his_lock.steps.get(&response.trace_id) {
                    Some(steps) => Some(steps.clone()),
                    None => None,
                }
            }
            None => None,
        }
    }

    pub fn get_request(&self, response: Response) -> Request {
        let steps = self.update_history(&response);
        Request {
            origin: response.origin,
            payload: response.payload,
            trace_id: response.trace_id,
            steps,
        }
    }

    pub fn get_by_reference(&self, reference: &str) -> Option<&Step> {
        match self.reference.get(reference) {
            Some(id) => self.steps.get(id),
            None => None,
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
        sender_request_runtime: Sender<PipelineRequest>,
    ) -> Result<(), ()> {
        let (sender_request_pipeline, receiver_request_pipeline): (
            Sender<Request>,
            Receiver<Request>,
        ) = mpsc::channel();

        if sender_setup_runtime
            .send(PipelineSetup {
                tx: sender_request_pipeline.clone(),
                id: self.id,
            })
            .is_err()
        {
            return Err(()); // TODO: definir
        }

        let (tx_senders, rx_senders): (Sender<BinSender>, Receiver<BinSender>) = mpsc::channel();
        let (tx_control, rx_control): (Sender<Response>, Receiver<Response>) = mpsc::channel();
        let mut module_id: ID = 0;

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

        let alias = modules.aliases.get(&self.key).unwrap().clone();

        let mut handler = Handler::new(sender_request_runtime.clone(), alias, self.key.clone());

        for step in self.pipe.pipeline.iter() {
            let step = step.clone();

            let current_module = match module_by_name.get(&step.module) {
                Some(module) => module,
                None => {
                    log::error!(r#"Module ¨"{}" not load in "{}""#, step.module, self.key);
                    continue;
                }
            };

            let reference = match step.reference {
                Some(reference) => reference,
                None => format!("step-{}", &module_id),
            };

            let mut params = match step.params {
                Some(params) => match params.as_object() {
                    Some(params) => params.clone(),
                    None => Map::new(),
                },
                None => Map::new(),
            };
            let mut module_setup_params = current_module.params.clone();
            let producer = step.tags.get("producer").is_some();
            let default_attach = step.attach;
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
                    &current_module.name,
                    StepConfig {
                        id,
                        reference: reference.clone(),
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
                    &current_module.name,
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
                let bin_key = modules.get_bin_key(&module_inner.key);
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

        let pipeline_traces: Arc<Mutex<HashMap<u32, bool>>> = Arc::new(Mutex::new(HashMap::new()));
        let handler_thread = handler.clone();
        let pipeline_traces_thread = pipeline_traces.clone();
        let key_thread = self.key.clone();

        thread::spawn(move || {
            for control in rx_control {
                let request = handler_thread.get_request(control.clone());

                if let Some(attach) = control.attach {
                    match handler_thread.get_by_reference(&attach) {
                        Some(step) => match step.send(request) {
                            Ok(_) => continue,
                            Err(err) => {
                                panic!("{:#?}", err);
                            }
                        },
                        None => {
                            panic!("Reference {} not found", attach);
                        }
                    };
                } else {
                    let next_step = control.origin + 1;

                    let mut lock_pipeline_traces = pipeline_traces_thread.lock().unwrap();

                    match lock_pipeline_traces.get(&next_step) {
                        Some(pipeline_trace) => {
                            if pipeline_trace == &false {
                                lock_pipeline_traces.insert(next_step, true);
                            } else {
                                match sender_request_runtime.send(PipelineRequest::from_request(
                                    next_step,
                                    key_thread.clone(),
                                    request,
                                )) {
                                    Ok(_) => continue,
                                    Err(err) => {
                                        panic!("{:#?}", err);
                                    }
                                };
                            }
                        }
                        None => {
                            match handler_thread.steps.get(&next_step) {
                                Some(step) => match step.send(request) {
                                    Ok(_) => continue,
                                    Err(err) => {
                                        panic!("{:#?}", err);
                                    }
                                },
                                None if control.origin > 0 => {
                                    match &handler_thread.steps.get(&0) {
                                        Some(step) => match step.send(request) {
                                            Ok(_) => continue,
                                            Err(err) => {
                                                panic!("{:#?}", err);
                                            }
                                        },
                                        None => {
                                            panic!(
                                                "trace_id: {} |  Sender by step id {} not exist",
                                                control.trace_id, next_step
                                            );
                                        }
                                    };
                                }
                                None => (),
                            };
                        }
                    }
                }
            }
        });

        for request in receiver_request_pipeline {
            let step = handler.steps.get(&0).unwrap();
            let trace_id = request.trace_id;

            pipeline_traces.lock().unwrap().insert(trace_id, false);

            match step.send(request) {
                Ok(_) => continue,
                Err(_) => {
                    panic!("trace_id: {} |  Sender by step id 0 not exist", trace_id);
                }
            }
        }

        Ok(())
    }
}
