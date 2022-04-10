use pipe_core::{
    debug, log,
    modules::{BinSender, Config, History, Request, Response, ID},
};

use std::sync::mpsc::{Receiver, Sender};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use std::{sync::mpsc, thread};

use crate::{
    pipe::Pipe,
    runtime::{Modules, PipelineResponse, PipelineSender},
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
        sender_module_global: Sender<PipelineSender>,
        _sender_response_global: Sender<PipelineResponse>,
    ) -> Result<(), ()> {
        let (sender_local, _receiver_local): (Sender<Request>, Receiver<Request>) = mpsc::channel();

        if sender_module_global
            .send(PipelineSender {
                tx: sender_local.clone(),
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

        for step in self.pipe.pipeline.iter() {
            let step = step.clone();
            let response = tx_control.clone();
            let request = tx_senders.clone();
            let module_name = step.module;
            let reference = match step.reference {
                Some(reference) => reference,
                None => format!("step-{}", &module_id),
            };

            modules_reference.add(reference.clone(), module_id.clone());

            let params = step.params;
            let producer = step.tags.get("producer").is_some();
            let default_attach = step.attach;
            let current_module = module_by_name.get(&module_name).unwrap().clone();
            let modules = modules.clone();
            let module_inner = modules.get(&self.key, &current_module.name);

            log::trace!(
                "Starting step {}, module_id: {}.",
                reference.clone(),
                module_id
            );

            {
                let module_id = module_id.clone();

                let tags = step.tags.clone();
                let args = step.args.clone();
                let reference = reference.clone();
                let module_params = current_module.params.clone();

                thread::spawn(move || {
                    let bin = modules.get_bin(&module_inner.name).extract();

                    bin.start(
                        module_id,
                        request,
                        response,
                        Config {
                            reference,
                            params,
                            producer,
                            default_attach,
                            tags,
                            module_params,
                            args,
                        },
                    );
                });
            }

            module_id = module_id + 1;
        }

        let mut senders = HashMap::new();
        debug!("INIT pos");

        for sender in rx_senders {
            senders.insert(sender.id, sender.tx);
            if (senders.len() as u32) == module_id {
                break;
            }
        }
        debug!("INIT ssss");

        let history = Arc::new(Mutex::new(History::new()));

        for control in rx_control {
            log::trace!(
                "trace_id: {} | Step {} sender: {:?}",
                control.trace_id,
                control.origin,
                control
            );

            let module_name = modules_reference.get_by_id(control.origin);
            let mut his_lock = history.lock().unwrap();

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

            match control.attach {
                Some(attach) => {
                    log::trace!(
                        "trace_id: {} | Resolving attach: {}",
                        control.trace_id,
                        attach.clone()
                    );
                    match senders.get(&modules_reference.get_by_name(&attach)) {
                        Some(module) => {
                            log::trace!(
                                "trace_id: {} | Sender from {} to step: {}",
                                control.trace_id,
                                control.origin,
                                module_id
                            );
                            match module.send(request) {
                                Ok(_) => log::trace!(
                                    "trace_id: {} | Sended from {} to step {}",
                                    control.trace_id,
                                    control.origin,
                                    module_id
                                ),
                                Err(err) => log::error!(
                                    "trace_id: {} |Send Error from {} to {}: {:?}",
                                    control.trace_id,
                                    control.origin,
                                    module_id,
                                    err
                                ),
                            } // TODO: Forçar retorno de erro para o step anterior
                        }
                        None => log::warn!(
                            "trace_id: {} | Reference {} not found",
                            control.trace_id,
                            attach
                        ),
                    };
                }
                None => {
                    let next_step = control.origin + 1;
                    log::trace!(
                        "trace_id: {} | Resolving next step id: {}",
                        control.trace_id,
                        next_step
                    );
                    match senders.get(&next_step) {
                        Some(module) => {
                            module.send(request).unwrap();
                        }
                        None if control.origin > 0 => {
                            log::trace!(
                                "trace_id: {} |  Step id {} not exist, send to step id 0",
                                control.trace_id,
                                next_step
                            );
                            senders.get(&0).unwrap().send(request).unwrap();
                        }
                        None => (),
                    };
                }
            }
        }

        Ok(())
    }
}
