use super::{step::StepConfig, Pipeline};
use crate::{lab::ModuleType, runtime::Modules};
use core::panic;
use lab_core::{
    log,
    modules::{BinSender, Module, PreConfig, Response, ID},
    params::Params,
};
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::thread;
use std::{
    convert::TryFrom,
    sync::mpsc::{Receiver, Sender},
};

impl Pipeline {
    pub fn load_and_start_steps<'a>(
        &mut self,
        initial_step_id: ID,
        modules: &Modules,
        sender_steps: Sender<Response>,
        sender_bin: Sender<BinSender>,
        receiver_bin: Receiver<BinSender>,
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

                self.pipeline_data.insert_pipeline(
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

                self.pipeline_data.insert_bin(
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

        self.wait_senders(receiver_bin);
    }

    pub fn wait_senders(&mut self, receiver_bin: Receiver<BinSender>) {
        if self.pipeline_data.total_bins > 0 {
            let mut limit_senders = self.pipeline_data.total_bins - 1;

            for sender in receiver_bin {
                self.pipeline_data.bin_sender(sender.id, sender.tx);

                if limit_senders == 0 {
                    break;
                }

                limit_senders -= 1;
            }
        }
    }
}
