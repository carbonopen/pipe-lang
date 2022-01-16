use libloading::{Library, Symbol};
use pipe_core::modules::{Config, Module, ModuleContact, Request, Response, ID};
use pipe_parser::value::Value;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::{sync::mpsc, thread};

fn load_modules(imports: &HashMap<String, Value>) -> HashMap<String, String> {
    let mut modules = HashMap::new();

    println!("{:?}", imports);

    modules
}

pub fn runtime(pipe: Value) -> Result<(), ()> {
    let pipe_obj = match pipe.to_object() {
        Ok(value) => value,
        Err(_) => return Err(()),
    };

    let modules = match pipe_obj.get("import") {
        Some(value) => match value.to_object() {
            Ok(imports) => load_modules(&imports),
            Err(_) => HashMap::default(),
        },
        None => HashMap::default(),
    };

    Ok(())
    /*
    let (tx_senders, rx_senders): (Sender<ModuleContact>, Receiver<ModuleContact>) =
        mpsc::channel();
    let (tx_control, rx_control): (Sender<Response>, Receiver<Response>) = mpsc::channel();
    let mut module_id: ID = 0;
    let mut references = HashMap::new();

    for step in pipe.pipeline {
        trace!("Load step: {:?}", step);
        let response = tx_control.clone();
        let request = tx_senders.clone();
        let module_name = step.module.unwrap_or("payload".to_string());
        let reference = match step.reference {
            Some(reference) => reference,
            None => format!("step-{}", &module_id),
        };
        references.insert(reference.clone(), module_id);
        let params = step.params;
        let producer = step.producer;
        let default_attach = step.attach;
        let filename = {
            let name = (**modules.get(&module_name).unwrap()).to_string();

            if cfg!(unix) && !name.contains(".so") {
                format!("{}.so", name)
            } else if cfg!(windows) && !name.contains(".dll") {
                format!("{}", name)
            } else {
                name
            }
        };

        trace!(
            "Starting step {}, module_id: {}.",
            reference.clone(),
            module_id
        );

        {
            let module_id = module_id.clone();

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
                    },
                );
            });
        }

        module_id = module_id + 1;
    }

    let mut senders = HashMap::new();

    for sender in rx_senders {
        trace!("Step {} started.", sender.id.clone());
        senders.insert(sender.id, sender.tx);
        if (senders.len() as u32) == module_id {
            break;
        }
    }

    for control in rx_control {
        trace!(
            "trace_id: {} | Step {} sender: {:?}",
            control.trace_id,
            control.origin,
            control
        );

        match control.attach {
            Some(attach) => {
                trace!(
                    "trace_id: {} | Resolving attach: {}",
                    control.trace_id,
                    attach.clone()
                );
                match references.get(&attach.clone()) {
                    Some(module_id) => match senders.get(&module_id) {
                        Some(module) => {
                            trace!(
                                "trace_id: {} | Sender to step: {}",
                                control.trace_id,
                                module_id
                            );
                            module
                                .send(Request {
                                    origin: control.origin,
                                    payload: control.payload,
                                    trace_id: control.trace_id,
                                })
                                .unwrap();
                        }
                        None => warn!("Reference {} not found", attach),
                    },
                    _ => warn!("Reference {} not found", attach),
                };
            }
            None => {
                let next_step = control.origin + 1;
                trace!(
                    "trace_id: {} | Resolving next step id: {}",
                    control.trace_id,
                    next_step
                );
                match senders.get(&next_step) {
                    Some(module) => {
                        module
                            .send(Request {
                                origin: control.origin,
                                payload: control.payload,
                                trace_id: control.trace_id,
                            })
                            .unwrap();
                    }
                    None if control.origin > 0 => {
                        trace!(
                            "trace_id: {} |  Step id {} not exist, send to step id 0",
                            control.trace_id,
                            next_step
                        );
                        senders
                            .get(&0)
                            .unwrap()
                            .send(Request {
                                origin: control.origin,
                                payload: control.payload,
                                trace_id: control.trace_id,
                            })
                            .unwrap();
                    }
                    None => (),
                };
            }
        }
    } */
}
