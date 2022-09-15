use core::panic;
use pipe_core::modules::{Request, ID};
use pipe_parser::Pipe as PipeParse;
use std::{collections::HashMap, fmt::Debug};
use std::{
    path::PathBuf,
    str::FromStr,
    sync::mpsc::{Receiver, Sender},
};
use std::{sync::mpsc, thread};

use crate::pipe::{ModuleType, Pipe};
use crate::pipeline::Pipeline;

#[derive(Debug, Clone)]
pub struct ModuleInner {
    pub module_type: ModuleType,
    pub key: String,
}

pub type Alias = HashMap<String, ModuleInner>;
pub type Pipelines = HashMap<String, Pipeline>;
pub type Aliases = HashMap<String, Alias>;
pub type Bins = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct Modules {
    pub bins: Bins,
    pub aliases: Aliases,
}

impl Modules {
    pub(crate) fn get(&self, owner: &str, alias: &str) -> ModuleInner {
        self.aliases.get(owner).unwrap().get(alias).unwrap().clone()
    }

    pub(crate) fn get_bin_key(&self, key: &str) -> String {
        self.bins.get(key).unwrap().clone()
    }
}

#[derive(Debug)]
pub struct PipelineTarget {
    pub id: u32,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct PipelineRequest {
    pub step_attach: Option<ID>,
    pub pipeline_attach: Option<ID>,
    pub request: Request,
    pub return_pipeline: bool,
}

impl PipelineRequest {
    pub fn from_request(
        request: Request,
        pipeline_attach: Option<ID>,
        step_attach: Option<ID>,
        return_pipeline: bool,
    ) -> Self {
        Self {
            request: Request {
                payload: request.payload,
                origin: request.origin,
                trace: request.trace,
                steps: request.steps.clone(),
            },
            step_attach,
            pipeline_attach,
            return_pipeline,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineSetup {
    pub tx: Sender<PipelineRequest>,
    pub id: u32,
}

#[derive(Debug)]
pub struct Runtime {
    pipelines: Pipelines,
    pipelines_keys: Vec<String>,
    modules: Modules,
    references: HashMap<String, ID>,
}

impl Runtime {
    pub fn builder(target: &str) -> Result<Self, ()> {
        let mut targets = vec![target.to_string()];
        let mut aliases: Aliases = HashMap::new();
        let mut pipelines: Pipelines = HashMap::new();
        let mut references = HashMap::new();
        let mut bins: Bins = HashMap::new();
        let mut pipelines_keys = Vec::new();
        let mut pipeline_id: ID = 0;

        loop {
            let index = if targets.len() > 0 {
                targets.len() - 1
            } else {
                break;
            };

            let path = PathBuf::from_str(targets.get(index).unwrap()).unwrap();
            let target = path.canonicalize().unwrap();
            let target_key = target.to_str().unwrap().to_string();

            let pipe = match PipeParse::from_path(&target_key) {
                Ok(value) => Pipe::new(&value),
                Err(_) => return Err(()),
            };

            let path_base = target.parent().unwrap().to_str().unwrap();

            let pipeline = Pipeline::new(pipeline_id, target_key.clone(), pipe.clone());
            pipelines_keys.push(target_key.clone());
            pipelines.insert(target_key.clone(), pipeline);
            references.insert(target_key.clone(), pipeline_id);

            for module in pipe.modules.unwrap().iter() {
                let path_raw = format!("{}/{}", path_base, module.path);
                let module_key = match PathBuf::from_str(&path_raw).unwrap().canonicalize() {
                    Ok(path) => path.to_str().unwrap().to_string(),
                    Err(err) => {
                        panic!("{}: {:?}", path_raw, err);
                    }
                };

                match aliases.get_mut(&target_key) {
                    Some(group) => {
                        group.insert(
                            module.name.clone(),
                            ModuleInner {
                                key: module_key.clone(),
                                module_type: module.module_type.clone(),
                            },
                        );
                    }
                    None => {
                        aliases.insert(target_key.clone(), {
                            let mut group: Alias = HashMap::new();

                            group.insert(
                                module.name.clone(),
                                ModuleInner {
                                    key: module_key.clone(),
                                    module_type: module.module_type.clone(),
                                },
                            );

                            group
                        });
                    }
                }

                if module.module_type.eq(&ModuleType::Bin) {
                    if bins.get(&module_key).is_none() {
                        bins.insert(module_key.clone(), module_key.clone());
                    }
                } else if module.module_type.eq(&ModuleType::Pipeline) {
                    if pipelines.get(&module_key).is_none() {
                        let new_target = format!("{}/{}", path_base, module.path);
                        targets.push(new_target)
                    }
                }
            }

            targets.remove(index);

            pipeline_id += 1;
        }

        Ok(Self {
            pipelines,
            modules: Modules { bins, aliases },
            pipelines_keys,
            references,
        })
    }

    pub fn start(&self) {
        let mut pipeline_steps_ref = HashMap::new();
        let mut pipeline_senders = HashMap::new();
        let (sender_control, receiver_control): (
            Sender<PipelineRequest>,
            Receiver<PipelineRequest>,
        ) = mpsc::channel();

        self.start_pipelines(
            sender_control,
            &mut pipeline_steps_ref,
            &mut pipeline_senders,
        );

        Self::listener(receiver_control, pipeline_steps_ref, pipeline_senders);
    }

    fn start_pipelines<'a>(
        &self,
        sender_control: Sender<PipelineRequest>,
        pipeline_steps_ref: &mut HashMap<u32, u32>,
        pipeline_senders: &mut HashMap<u32, Sender<PipelineRequest>>,
    ) {
        {
            let (sender_pipeline, receiver_pipeline): (
                Sender<PipelineSetup>,
                Receiver<PipelineSetup>,
            ) = mpsc::channel();

            let pipes = self.pipelines.clone();
            let modules = self.modules.clone();
            let mut last_steps_id: ID = 0;

            for key in self.pipelines_keys.iter() {
                let mut pipeline = pipes.get(key).unwrap().clone();
                let modules = modules.clone();
                let sender_pipeline = sender_pipeline.clone();
                let sender_control = sender_control.clone();
                let initial_step_id = last_steps_id;
                last_steps_id += pipeline.pipe.pipeline.len() as ID;

                for step_id in initial_step_id..last_steps_id {
                    pipeline_steps_ref.insert(step_id, pipeline.id);
                }

                pipeline.add_references(self.references.clone());

                thread::spawn(move || {
                    match pipeline.start(
                        modules.clone(),
                        sender_pipeline,
                        sender_control,
                        initial_step_id,
                    ) {
                        Ok(_) => (),
                        Err(_) => panic!("Pipeline Error: {}", pipeline.key),
                    };
                });
            }

            let mut pipelines_done = self.pipelines_keys.len() - 1;

            for pipeline_sender in receiver_pipeline {
                pipeline_senders.insert(pipeline_sender.id, pipeline_sender.tx);

                if pipelines_done == 0 {
                    break;
                }

                pipelines_done -= 1;
            }
        }
    }

    fn listener(
        receiver_control: Receiver<PipelineRequest>,
        pipeline_steps_ref: HashMap<u32, u32>,
        pipeline_senders: HashMap<u32, Sender<PipelineRequest>>,
    ) {
        for pipeline_request in receiver_control {
            let pipeline_id = match pipeline_request.pipeline_attach {
                Some(id) => id,
                None => pipeline_steps_ref
                    .get(&pipeline_request.step_attach.unwrap())
                    .unwrap()
                    .clone(),
            };

            let origin_pipeline = pipeline_steps_ref
                .get(&pipeline_request.request.origin)
                .unwrap();

            let (new_pipeline_request, pipeline_id) = if pipeline_id.eq(origin_pipeline) {
                (
                    PipelineRequest {
                        return_pipeline: true,
                        ..pipeline_request
                    },
                    pipeline_id,
                )
            } else {
                (pipeline_request, pipeline_id)
            };

            let sender = pipeline_senders.get(&pipeline_id).unwrap();

            match sender.send(new_pipeline_request) {
                Ok(_) => continue,
                Err(err) => panic!("{:?}", err),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_tet() {
        match Runtime::builder("demo/modules/main.pipe") {
            Ok(run) => run.start(),
            Err(_) => assert!(false),
        }
    }
}
