use libloading::{Library, Symbol};
use pipe_core::{
    log,
    modules::{Config, History, Module, ModuleContact, Request, Response, ID},
};
use pipe_parser::{Error as PipeParseError, Pipe as PipeParse};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use std::{
    path::PathBuf,
    str::FromStr,
    sync::mpsc::{Receiver, Sender},
};
use std::{sync::mpsc, thread};

use crate::pipe::{ModuleType, Pipe};

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

struct ModuleInner {
    name: String,
    bin: Option<Box<dyn Module>>,
    module_type: ModuleType,
    aliases: HashMap<String, Vec<String>>,
}

impl Debug for ModuleInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleInner")
            .field("name", &self.name)
            .field("module_type", &self.module_type)
            .field("aliases", &self.aliases)
            .finish()
    }
}

impl ModuleInner {
    pub(crate) fn new(
        name: String,
        module_type: ModuleType,
        bin: Option<Box<dyn Module>>,
        alias: Option<(String, String)>,
    ) -> Self {
        let aliases = if let Some(alias) = alias {
            let mut aliases = HashMap::new();
            aliases.insert(alias.0, vec![alias.1]);
            aliases
        } else {
            HashMap::default()
        };

        Self {
            name,
            bin,
            module_type,
            aliases,
        }
    }

    pub(crate) fn add_alias(&mut self, owner: String, alias: String) {
        match self.aliases.get_mut(&owner) {
            Some(aliases) => {
                aliases.push(alias);
            }
            None => {
                self.aliases.insert(owner, vec![alias]);
            }
        }
    }
}

type Modules = HashMap<String, ModuleInner>;
type Pipelines = HashMap<String, Pipeline>;
pub struct Runtime {
    pipelines: Pipelines,
    modules: Modules,
    main: String,
}

impl Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("pipelines", &self.pipelines)
            .field("modules", &self.modules)
            .field("main", &self.main)
            .finish()
    }
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

    fn extract(target: &str) -> Result<(Modules, Pipelines, String), PipeParseError> {
        let mut targets = vec![target.to_string()];
        let mut modules: HashMap<String, ModuleInner> = HashMap::new();
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

            let path_base = target.parent().unwrap().to_str().unwrap();

            pipelines.insert(target_string.clone(), Pipeline::new(pipe.clone()));

            for module in pipe.modules.unwrap().iter() {
                let dir = format!("{}/{}", path_base, module.path);
                println!("{:?}", dir);
                println!("");
                let module_key = PathBuf::from_str(&dir)
                    .unwrap()
                    .canonicalize()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();

                if module.module_type.eq(&ModuleType::Bin) {
                    let lib = match Library::new(module_key.clone()) {
                        Ok(lib) => lib,
                        Err(err) => panic!("Error: {}; Filename: {}", err, module_key.clone()),
                    };
                    let bin = unsafe {
                        let constructor: Symbol<unsafe extern "C" fn() -> *mut dyn Module> =
                            lib.get(b"_Module").unwrap();
                        let boxed_raw = constructor();
                        Box::from_raw(boxed_raw)
                    };

                    match modules.get_mut(&module_key) {
                        Some(module_inner) => {
                            module_inner.add_alias(target_string.clone(), module.name.clone());
                        }
                        None => {
                            let inner = ModuleInner::new(
                                module_key.clone(),
                                module.module_type.clone(),
                                Some(bin),
                                Some((target_string.clone(), module.name.clone())),
                            );

                            modules.insert(module_key.clone(), inner);
                        }
                    }
                } else if module.module_type.eq(&ModuleType::Pipeline) {
                    let inner = ModuleInner::new(
                        module_key.clone(),
                        module.module_type.clone(),
                        None,
                        Some((target_string.clone(), module.name.clone())),
                    );

                    modules.insert(module_key.clone(), inner);

                    if pipelines.get(&module_key).is_none() {
                        let new_target = format!("{}/{}", path_base, module.path);
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
    module_by_name: HashMap<String, crate::pipe::Module>,
}

impl Pipeline {
    pub(crate) fn new(pipe: Pipe) -> Self {
        let module_by_name = {
            let mut modules = HashMap::new();
            for module in pipe.modules.clone().unwrap() {
                modules.insert(module.name.clone(), module.clone());
            }
            modules
        };

        Self {
            pipe,
            module_by_name,
        }
    }

    pub fn start(&self) {
        println!("{:#?}", self);
    }

    pub fn run(&self) {
        let (tx_senders, rx_senders): (Sender<ModuleContact>, Receiver<ModuleContact>) =
            mpsc::channel();
        let (tx_control, rx_control): (Sender<Response>, Receiver<Response>) = mpsc::channel();
        let mut module_id: ID = 0;
        let mut modules_reference = Reference::default();

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
            let current_module = self.module_by_name.get(&module_name).unwrap().clone();

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
                    // module.start(
                    //     module_id,
                    //     request,
                    //     response,
                    //     Config {
                    //         reference: reference.clone(),
                    //         params,
                    //         producer,
                    //         default_attach,
                    //         tags,
                    //         module_params: current_module.params.clone(),
                    //         args,
                    //     },
                    // );
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
}
