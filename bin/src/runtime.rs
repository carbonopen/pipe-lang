use libloading::{Library, Symbol};
use pipe_core::{
    log,
    modules::{Config, History, Module as PipeModule, ModuleContact, Request, Response, ID},
};
use pipe_parser::{Error as PipeParseError, Pipe as PipeParse};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use std::{
    path::PathBuf,
    str::FromStr,
    sync::mpsc::{Receiver, Sender},
};
use std::{sync::mpsc, thread};

use crate::pipe::{Module, ModuleType, Pipe};

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
pub struct Runtime {
    pipelines: HashMap<String, Pipeline>,
    modules: HashMap<String, Module>,
    main: String,
}

impl Runtime {
    pub fn builder(main: &str) -> Result<Self, ()> {
        let (modules, pipelines, main) = match Self::extract(main) {
            Ok(value) => value,
            Err(_) => return Err(()),
        };

        Ok(Self {
            pipelines,
            modules,
            main,
        })
    }

    fn extract(
        target: &str,
    ) -> Result<(HashMap<String, Module>, HashMap<String, Pipeline>, String), PipeParseError> {
        let mut targets = vec![target.to_string()];
        let mut modules = HashMap::new();
        let mut pipelines = HashMap::new();
        let main = PathBuf::from_str(target)
            .unwrap()
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        loop {
            let index = if targets.len() > 0 {
                targets.len() - 1
            } else {
                break;
            };

            let path = PathBuf::from_str(targets.get(index).unwrap()).unwrap();
            let target = path.canonicalize().unwrap();
            let target_string = target.to_str().unwrap().to_string();

            let pipe = match PipeParse::from_path(&target_string) {
                Ok(value) => Pipe::new(&value),
                Err(err) => return Err(err),
            };

            pipelines.insert(target_string.clone(), Pipeline::new(pipe.clone()));

            for module in pipe.modules.unwrap() {
                if module.module_type.eq(&ModuleType::Bin) {
                    if modules.get(&module.name).is_none() {
                        modules.insert(module.name.clone(), module.clone());
                    }
                } else if module.module_type.eq(&ModuleType::Pipeline) {
                    if pipelines.get(&module.name).is_none() {
                        let new_target = format!(
                            "{}/{}.pipe",
                            path.parent().unwrap().to_str().unwrap(),
                            module.path
                        );
                        targets.push(new_target)
                    }
                }
            }

            targets.remove(index);

            if targets.len() == 0 {
                break;
            }
        }

        Ok((modules, pipelines, main))
    }

    fn get_main(&self) -> &Pipeline {
        self.pipelines.get(&self.main).unwrap()
    }

    pub fn start(&self) {
        println!("START");
        println!("{:#?}", self);
    }
}

#[derive(Debug)]
struct Pipeline {
    pipe: Pipe,
}

impl Pipeline {
    pub(crate) fn new(pipe: Pipe) -> Self {
        Self { pipe }
    }
}

pub fn pipeline(pipe: Pipe) {
    let modules = {
        let mut modules = HashMap::new();
        for module in pipe.modules.unwrap() {
            log::trace!("Module: {:?}", module);
            modules.insert(module.name.clone(), module.clone());
        }
        modules
    };

    let (tx_senders, rx_senders): (Sender<ModuleContact>, Receiver<ModuleContact>) =
        mpsc::channel();
    let (tx_control, rx_control): (Sender<Response>, Receiver<Response>) = mpsc::channel();
    let mut module_id: ID = 0;
    let mut modules_reference = Reference::default();

    for step in pipe.pipeline {
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
        let current_module = modules.get(&module_name).unwrap().clone();

        let filename = {
            let name = current_module.path.to_string();

            if cfg!(unix) && !name.contains(".so") {
                format!("{}.so", name)
            } else if cfg!(windows) && !name.contains(".dll") {
                format!("{}", name)
            } else {
                name
            }
        };

        log::trace!(
            "Starting step {}, module_id: {}.",
            reference.clone(),
            module_id
        );

        {
            let module_id = module_id.clone();

            let tags = step.tags.clone();
            let args = step.args.clone();

            thread::spawn(move || {
                let lib = match Library::new(filename.clone()) {
                    Ok(lib) => lib,
                    Err(err) => panic!("Error: {}; Filename: {}", err, filename),
                };
                let module = unsafe {
                    let constructor: Symbol<unsafe extern "C" fn() -> *mut dyn PipeModule> =
                        lib.get(b"_Module").unwrap();
                    let boxed_raw = constructor();
                    Box::from_raw(boxed_raw)
                };

                module.start(
                    module_id,
                    request,
                    response,
                    Config {
                        reference: reference.clone(),
                        params,
                        producer,
                        default_attach,
                        tags,
                        module_params: current_module.params.clone(),
                        args,
                    },
                );
            });
        }

        module_id = module_id + 1;
    }

    let mut senders = HashMap::new();

    for sender in rx_senders {
        log::trace!("Step {} started.", sender.id.clone());
        senders.insert(sender.id, sender.tx);
        if (senders.len() as u32) == module_id {
            break;
        }
    }

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
}
