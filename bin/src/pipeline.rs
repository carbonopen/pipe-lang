use libloading::{Library, Symbol};
use pipe_core::{
    log,
    modules::{BinSender, Config, History, Module, Request, Response, Trace, ID},
};
use serde_json::{Map, Value};

use core::panic;
use std::sync::mpsc::{Receiver, Sender};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use std::{sync::mpsc, thread};

use crate::{
    pipe::{ModuleType, Pipe},
    runtime::{Alias, Modules, PipelineRequest, PipelineSetup},
};

pub type Params = Map<String, Value>;

#[derive(Debug, Clone)]
pub struct StepConfig {
    pub id: ID,
    pub pipeline_id: ID,
    pub reference: String,
    pub params: Params,
    pub producer: bool,
    pub default_attach: Option<String>,
    pub tags: HashMap<String, Value>,
    pub args: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
struct Step {
    pub id: ID,
    pub pipeline_id: ID,
    pub module_type: ModuleType,
    pub sender: Option<Sender<Request>>,
    pub config: StepConfig,
    pub sender_pipeline: Option<Sender<PipelineRequest>>,
    pub params: Params,
}

impl Step {
    pub fn send(&self, mut request: Request) -> Result<(), ()> {
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
                    let mut args = HashMap::new();

                    for (key, value) in self.params.iter() {
                        args.insert(key.clone(), value.clone());
                    }

                    request.set_args(args);

                    match sender.send(PipelineRequest::from_request(
                        request,
                        Some(self.pipeline_id),
                        None,
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
    pub steps: HashMap<u32, Step>,
    pub history: Arc<Mutex<History>>,
    pub reference: HashMap<String, u32>,
    total_bins: u32,
    pipeline_sender: Sender<PipelineRequest>,
}

impl Handler {
    pub fn new(pipeline_sender: Sender<PipelineRequest>) -> Self {
        Self {
            steps: HashMap::default(),
            history: Arc::new(Mutex::new(History::new())),
            total_bins: 0,
            reference: HashMap::default(),
            pipeline_sender,
        }
    }

    pub fn insert_pipeline(&mut self, id: ID, pipeline_id: ID, config: StepConfig) {
        self.reference.insert(config.reference.clone(), id);

        self.steps.insert(
            id,
            Step {
                id,
                pipeline_id,
                module_type: ModuleType::Pipeline,
                sender: None,
                params: config.params.clone(),
                config,
                sender_pipeline: Some(self.pipeline_sender.clone()),
            },
        );
    }

    pub fn insert_bin(&mut self, id: ID, pipeline_id: ID, config: StepConfig) {
        self.reference.insert(config.reference.clone(), id);

        self.steps.insert(
            id,
            Step {
                id,
                pipeline_id,
                module_type: ModuleType::Bin,
                sender: None,
                params: config.params.clone(),
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
                    response.trace.clone(),
                    step.config.reference.clone(),
                    response.clone(),
                );

                match his_lock.steps.get(&response.trace.trace_id) {
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
            steps,
            trace: response.trace,
        }
    }

    pub fn get_by_reference(&self, reference: &str) -> Option<&Step> {
        match self.reference.get(reference) {
            Some(id) => self.steps.get(id),
            None => None,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
enum Status {
    Created,
    Sended,
}

#[derive(Debug)]
struct PipelineTrace {
    pub status: Status,
    pub pipeline_request: PipelineRequest,
}

impl PipelineTrace {
    pub fn update_status(&mut self, status: Status) {
        self.status = status
    }
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub id: u32,
    pub key: String,
    pub pipe: Pipe,
    pub references: HashMap<String, ID>,
}

impl Pipeline {
    pub fn new(id: u32, key: String, pipe: Pipe) -> Self {
        Self {
            id,
            key,
            pipe,
            references: HashMap::default(),
        }
    }

    pub fn add_references(&mut self, references: HashMap<String, ID>) {
        self.references = references;
    }

    pub fn start(
        &self,
        modules: Modules,
        sender_setup_runtime: Sender<PipelineSetup>,
        sender_request_runtime: Sender<PipelineRequest>,
        initial_step_id: ID,
    ) -> Result<(), ()> {
        let (sender_request_pipeline, receiver_request_pipeline): (
            Sender<PipelineRequest>,
            Receiver<PipelineRequest>,
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
        let mut step_id = initial_step_id;

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

        let mut handler = Handler::new(sender_request_runtime.clone());

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
                None => format!("step-{}", &step_id),
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
            let tags = step.tags.clone();
            let args = step.args.clone();
            let reference = reference.clone();

            if module_inner.module_type.eq(&ModuleType::Pipeline) {
                module_setup_params.remove("name");
                module_setup_params.remove("mod");

                for (key, value) in module_setup_params {
                    params.insert(key, value);
                }

                let pipeline_id = *self.references.get(&module_inner.key).unwrap();

                handler.insert_pipeline(
                    step_id,
                    pipeline_id,
                    StepConfig {
                        id: step_id,
                        pipeline_id,
                        reference: reference.clone(),
                        params,
                        producer,
                        default_attach,
                        args,
                        tags,
                    },
                );
            } else if module_inner.module_type.eq(&ModuleType::Bin) {
                module_setup_params.remove("name");
                module_setup_params.remove("bin");

                for (key, value) in module_setup_params {
                    params.insert(key, value);
                }

                handler.insert_bin(
                    step_id,
                    self.id,
                    StepConfig {
                        id: step_id,
                        pipeline_id: self.id,
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

                    bin.start(step_id, request, response, config);
                });
            }

            step_id += 1;
        }

        let mut limit_senders = if handler.total_bins > 0 {
            handler.total_bins - 1
        } else {
            0
        };

        for sender in rx_senders {
            handler.bin_sender(sender.id, sender.tx);

            if limit_senders == 0 {
                break;
            }

            limit_senders -= 1;
        }

        let pipeline_traces: Arc<Mutex<HashMap<u32, PipelineTrace>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let handler_thread = handler.clone();
        let pipeline_traces_thread = pipeline_traces.clone();

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

                    match handler_thread.steps.get(&next_step) {
                        Some(step) => match step.send(request) {
                            Ok(_) => continue,
                            Err(err) => {
                                panic!("{:#?}", err);
                            }
                        },
                        None if control.origin > 0 => {
                            let mut lock_pipeline_traces = pipeline_traces_thread.lock().unwrap();

                            match lock_pipeline_traces.get_mut(&control.trace.trace_id) {
                                Some(pipeline_trace) => match pipeline_trace.status {
                                    Status::Created => {
                                        match sender_request_runtime.send(
                                            PipelineRequest::from_request(
                                                request,
                                                None,
                                                Some(
                                                    pipeline_trace.pipeline_request.request.origin
                                                        + 2,
                                                ),
                                            ),
                                        ) {
                                            Ok(_) => {
                                                pipeline_trace.update_status(Status::Sended);
                                            }
                                            Err(err) => {
                                                panic!("{:#?}", err);
                                            }
                                        };
                                    }
                                    Status::Sended => {
                                        panic!("{}", "Message already sent");
                                    }
                                },
                                None => panic!("{}", "Step not found"),
                            }
                        }
                        None => match &handler_thread.steps.get(&0) {
                            Some(step) => match step.send(request) {
                                Ok(_) => continue,
                                Err(err) => {
                                    panic!("{:#?}", err);
                                }
                            },
                            None => {
                                panic!(
                                    "trace_id: {} |  Sender by step id {} not exist",
                                    control.trace.trace_id, next_step
                                );
                            }
                        },
                    };
                }
            }
        });

        for pipeline_request in receiver_request_pipeline {
            let step_id = match pipeline_request.step_attach {
                Some(step_attach) => step_attach,
                None => initial_step_id,
            };
            let step = handler.steps.get(&step_id).unwrap();
            let trace_id = pipeline_request.request.trace.trace_id;

            pipeline_traces.lock().unwrap().insert(
                trace_id,
                PipelineTrace {
                    status: Status::Created,
                    pipeline_request: pipeline_request.clone(),
                },
            );

            match step.send(pipeline_request.request) {
                Ok(_) => continue,
                Err(_) => {
                    panic!("trace_id: {} |  Sender by step id 0 not exist", trace_id);
                }
            }
        }

        Ok(())
    }
}
