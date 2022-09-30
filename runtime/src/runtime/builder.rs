use core::panic;
use lab_core::modules::ID;
use lab_parser::Lab as LabParse;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{path::PathBuf, str::FromStr};

use crate::envs::Envs;
use crate::lab::{Lab, ModuleType};
use crate::pipeline::Pipeline;
use crate::trace::{DebugTrace, PipelineTrace};

use super::{Alias, Aliases, Bins, ModuleInner, Modules, Pipelines, Runtime};

impl Runtime {
    pub fn builder(target: &str, envs: &Envs) -> Result<Self, ()> {
        let mut targets = vec![target.to_string()];
        let mut aliases: Aliases = HashMap::new();
        let mut pipelines: Pipelines = HashMap::new();
        let mut references = HashMap::new();
        let mut bins: Bins = HashMap::new();
        let mut pipelines_keys = Vec::new();
        let mut pipeline_id: ID = 0;
        let pipeline_traces = Arc::new(Mutex::new(PipelineTrace::new()));
        let debug_trace = DebugTrace::new(envs.debug_size_limit, envs.debug_enabled);

        loop {
            let index = targets.len() - 1;
            let path = PathBuf::from_str(targets.get(index).unwrap()).unwrap();
            let target = {
                let mut target = path.canonicalize().unwrap();

                if target.is_dir() {
                    target.push("main.lab")
                }

                target
            };

            let target_key = target.to_str().unwrap().to_string();

            let lab = match LabParse::from_path(&target_key) {
                Ok(value) => Lab::new(&value, &envs.runtime_extension_path),
                Err(_) => return Err(()),
            };

            let path_base = target.parent().unwrap().to_str().unwrap();

            let pipeline = Pipeline::new(
                pipeline_id,
                target_key.clone(),
                lab.clone(),
                pipeline_traces.clone(),
                debug_trace.clone(),
            );

            pipelines_keys.push(target_key.clone());
            pipelines.insert(target_key.clone(), pipeline);
            references.insert(target_key.clone(), pipeline_id);

            for module in lab.modules.unwrap().iter() {
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

            if targets.is_empty() {
                break;
            }

            pipeline_id += 1;
        }

        Ok(Self {
            pipelines,
            modules: Modules { bins, aliases },
            pipelines_keys,
            references,
        })
    }
}
