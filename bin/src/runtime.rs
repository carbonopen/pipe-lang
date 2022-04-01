use libloading::{Library, Symbol};
use pipe_core::{
    log,
    modules::{Config, History, Module, ModuleContact, Request, Response, ID},
};
use pipe_parser::value::Value;
use std::convert::TryFrom;
use std::sync::mpsc::{Receiver, Sender};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use std::{sync::mpsc, thread};

use crate::pipe::Pipe;

pub fn runtime(value: Value) {
    let pipe = Pipe::try_from(&value).expect("Could not capture code");
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
    let mut reference_id = HashMap::new();
    let mut id_reference = HashMap::new();

    for step in pipe.pipeline {
        let response = tx_control.clone();
        let request = tx_senders.clone();
        let module_name = step.module;
        let reference = match step.reference {
            Some(reference) => reference,
            None => format!("step-{}", &module_id),
        };
        reference_id.insert(reference.clone(), module_id);
        id_reference.insert(module_id, reference.clone());
        let params = step.params;
        let producer = step.tags.get("producer").is_some();
        let default_attach = step.attach;
        let current_module = modules.get(&module_name).unwrap().clone();

        let filename = {
            let name = current_module.bin.to_string();

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
            let vars = step.vars.clone();

            thread::spawn(move || {
                let lib = match Library::new(filename.clone()) {
                    Ok(lib) => lib,
                    Err(err) => panic!("Error: {}; Filename: {}", err, filename),
                };
                let module = unsafe {
                    let constructor: Symbol<unsafe extern "C" fn() -> *mut dyn Module> =
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
                        vars,
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

    //TODO: criar uma arvore contendo todos os payloads e passar como "module"
    //TODO: criar tags

    let history = Arc::new(Mutex::new(History::new()));

    for control in rx_control {
        log::trace!(
            "trace_id: {} | Step {} sender: {:?}",
            control.trace_id,
            control.origin,
            control
        );

        let module_name = id_reference.get(&control.origin).unwrap().to_string();
        let mut his_lock = history.lock().unwrap();

        his_lock.insert(control.trace_id, module_name, control.clone());

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
                match reference_id.get(&attach.clone()) {
                    Some(module_id) => match senders.get(&module_id) {
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
                    },
                    _ => log::warn!(
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
