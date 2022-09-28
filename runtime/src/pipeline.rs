use lab_core::{
    log,
    modules::{BinSender, History, Module, PreConfig, Request, Response, ID},
    params::Params,
};
use libloading::{Library, Symbol};

use core::panic;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use std::{
    convert::TryFrom,
    sync::mpsc::{Receiver, Sender},
};
use std::{sync::mpsc, thread};

use crate::{
    lab::{Lab, ModuleType},
    runtime::{Modules, PipelineRequest, PipelineSetup},
    step::{Step, StepConfig},
    trace::{DebugTrace, PipelineTrace},
};

#[derive(Debug, Clone)]
struct PipelineData {
    pub steps: HashMap<u32, Step>,
    pub history: Arc<Mutex<History>>,
    pub reference: HashMap<String, u32>,
    total_bins: u32,
    pipeline_sender: Sender<PipelineRequest>,
    debug_trace: DebugTrace,
}

impl PipelineData {
    pub fn new(pipeline_sender: Sender<PipelineRequest>, debug_trace: DebugTrace) -> Self {
        Self {
            steps: HashMap::default(),
            history: Arc::new(Mutex::new(History::new())),
            total_bins: 0,
            reference: HashMap::default(),
            pipeline_sender,
            debug_trace,
        }
    }

    pub fn insert_pipeline(&mut self, id: ID, pipeline_id: ID, config: StepConfig) {
        self.reference.insert(config.reference.clone(), id);

        self.steps.insert(
            id,
            Step {
                pipeline_id,
                module_type: ModuleType::Pipeline,
                sender: None,
                params: config.params.clone(),
                config,
                sender_pipeline: Some(self.pipeline_sender.clone()),
                debug_trace: self.debug_trace.clone(),
            },
        );
    }

    pub fn insert_bin(&mut self, id: ID, pipeline_id: ID, config: StepConfig) {
        self.reference.insert(config.reference.clone(), id);

        self.steps.insert(
            id,
            Step {
                pipeline_id,
                module_type: ModuleType::Bin,
                sender: None,
                params: config.params.clone(),
                config,
                sender_pipeline: None,
                debug_trace: self.debug_trace.clone(),
            },
        );
        self.total_bins += 1;
    }

    pub fn bin_sender(&mut self, id: ID, sender: Sender<Request>) {
        match self.steps.get_mut(&id) {
            Some(step) => step.sender = Some(sender),
            None => (),
        }
    }

    pub fn update_history(
        &self,
        response: &Response,
    ) -> Option<HashMap<String, lab_core::modules::Step>> {
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

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub id: ID,
    pub key: String,
    pub lab: Lab,
    pub references: HashMap<String, ID>,
    pipeline_traces: Arc<Mutex<PipelineTrace>>,
    debug_trace: DebugTrace,
}

impl Pipeline {
    pub fn new(
        id: ID,
        key: String,
        lab: Lab,
        pipeline_traces: Arc<Mutex<PipelineTrace>>,
        debug_trace: DebugTrace,
    ) -> Self {
        Self {
            id,
            key,
            lab,
            references: HashMap::default(),
            pipeline_traces,
            debug_trace,
        }
    }

    pub fn add_references(&mut self, references: HashMap<String, ID>) {
        self.references = references;
    }

    pub fn start(
        &mut self,
        modules: Modules,
        sender_setup_runtime: Sender<PipelineSetup>,
        sender_pipelines: Sender<PipelineRequest>,
        initial_step_id: ID,
    ) -> Result<(), ()> {
        let (sender_request_pipeline, receiver_pipelines): (
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
            panic!("An error occurred while starting the pipeline.");
        }

        drop(sender_setup_runtime);

        let mut pipeline_data =
            PipelineData::new(sender_pipelines.clone(), self.debug_trace.clone());
        let (sender_steps, receiver_steps): (Sender<Response>, Receiver<Response>) =
            mpsc::channel();

        {
            let (sender_bin, receiver_bin): (Sender<BinSender>, Receiver<BinSender>) =
                mpsc::channel();

            self.load_and_start_steps(
                initial_step_id,
                &modules,
                &mut pipeline_data,
                sender_steps.clone(),
                sender_bin.clone(),
            );

            Self::wait_senders(&mut pipeline_data, receiver_bin);

            let pipeline_data_thread = pipeline_data.clone();

            self.listener_step(
                receiver_steps,
                pipeline_data_thread,
                sender_pipelines,
                initial_step_id,
            );
        }

        self.listener_pipeline(
            receiver_pipelines,
            pipeline_data,
            initial_step_id,
            sender_steps,
        );

        Ok(())
    }

    fn load_and_start_steps<'a>(
        &self,
        initial_step_id: ID,
        modules: &Modules,
        pipeline_data: &mut PipelineData,
        sender_steps: Sender<Response>,
        sender_bin: Sender<BinSender>,
    ) {
        let module_by_name = match self.lab.modules.clone() {
            Some(modules) => {
                let mut result = HashMap::new();

                for module in modules.iter() {
                    result.insert(module.name.clone(), module.clone());
                }

                result
            }
            None => HashMap::default(),
        };

        for (index, step) in self.lab.pipeline.iter().enumerate() {
            let step_id = initial_step_id + index as ID;
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
                    Some(params) => params
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<HashMap<_, _>>(),
                    None => HashMap::new(),
                },
                None => HashMap::new(),
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

                pipeline_data.insert_pipeline(
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

                pipeline_data.insert_bin(
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

                let response = sender_steps.clone();
                let request = sender_bin.clone();
                let bin_key = modules.get_bin_key(&module_inner.key);

                thread::spawn(move || {
                    let pre_config = PreConfig {
                        reference,
                        params: Params::try_from(params).unwrap(),
                        producer,
                        default_attach,
                        tags,
                        args,
                    };

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

                    bin.start(step_id, request, response, pre_config);
                });
            }
        }
    }

    fn wait_senders(pipeline_data: &mut PipelineData, receiver_bin: Receiver<BinSender>) {
        if pipeline_data.total_bins > 0 {
            let mut limit_senders = pipeline_data.total_bins - 1;

            for sender in receiver_bin {
                pipeline_data.bin_sender(sender.id, sender.tx);

                if limit_senders == 0 {
                    break;
                }

                limit_senders -= 1;
            }
        }
    }

    fn listener_step<'a>(
        &self,
        receiver_steps: Receiver<Response>,
        pipeline_data: PipelineData,
        sender_pipelines: Sender<PipelineRequest>,
        initial_step_id: ID,
    ) {
        let pipeline_id = self.id;
        let pipeline_traces = self.pipeline_traces.clone();

        thread::spawn(move || {
            for response in receiver_steps {
                let request = pipeline_data.get_request(response.clone());

                if let Some(attach) = response.attach {
                    match pipeline_data.get_by_reference(&attach) {
                        Some(step) => match step.send(request) {
                            Ok(_) => continue,
                            Err(err) => {
                                panic!("{}", err);
                            }
                        },
                        None => {
                            panic!("Reference {} not found", attach);
                        }
                    };
                } else {
                    let next_step = response.origin + 1;

                    match pipeline_data.steps.get(&next_step) {
                        Some(step) => match step.send(request) {
                            Ok(_) => continue,
                            Err(err) => {
                                panic!("{}", err);
                            }
                        },
                        None => {
                            let mut lock_pipeline_traces = pipeline_traces.lock().unwrap();

                            match lock_pipeline_traces
                                .get_trace(&pipeline_id, &response.trace.trace_id)
                            {
                                Some(pipeline_request) => {
                                    match sender_pipelines.send(PipelineRequest::from_request(
                                        request,
                                        None,
                                        Some(pipeline_request.request.origin),
                                        true,
                                    )) {
                                        Ok(_) => {
                                            lock_pipeline_traces.remove_trace(
                                                &pipeline_id,
                                                &response.trace.trace_id,
                                            );
                                        }
                                        Err(err) => {
                                            panic!("{:#?}", err);
                                        }
                                    };
                                }
                                None => {
                                    match &pipeline_data.steps.get(&initial_step_id) {
                                        Some(step) => match step.send(request) {
                                            Ok(_) => continue,
                                            Err(err) => {
                                                panic!("{}", err);
                                            }
                                        },
                                        None => {
                                            panic!(
                                                "trace_id: {} |  Sender by step id {} not exist",
                                                response.trace.trace_id, next_step
                                            );
                                        }
                                    };
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    fn listener_pipeline(
        &mut self,
        receiver_pipelines: Receiver<PipelineRequest>,
        pipeline_data: PipelineData,
        initial_step_id: ID,
        sender_steps: Sender<Response>,
    ) {
        for pipeline_request in receiver_pipelines {
            let step_id = match pipeline_request.step_attach {
                Some(step_attach) if pipeline_request.return_pipeline == true => {
                    let origin = step_attach + 1;
                    let attach = pipeline_data
                        .steps
                        .get(&origin)
                        .unwrap()
                        .config
                        .default_attach
                        .clone();

                    let response = Response {
                        payload: pipeline_request.request.payload,
                        attach,
                        origin,
                        trace: pipeline_request.request.trace,
                    };

                    match sender_steps.send(response) {
                        Ok(_) => continue,
                        Err(_) => panic!("Return error"),
                    }
                }
                Some(step_attach) => step_attach,
                None => initial_step_id,
            };
            let step = match pipeline_data.steps.get(&step_id) {
                Some(step) => step,
                None => panic!("Step not found"),
            };
            let trace_id = pipeline_request.request.trace.trace_id;

            match step.send(pipeline_request.request.clone()) {
                Ok(_) if pipeline_request.return_pipeline == false => {
                    self.pipeline_traces.lock().unwrap().add_trace(
                        &self.id,
                        &trace_id,
                        pipeline_request,
                    );
                }
                Err(_) => {
                    panic!("trace_id: {} |  Sender by step id 0 not exist", trace_id);
                }
                _ => (),
            }
        }
    }
}
