use std::{collections::HashMap, sync::mpsc, thread};

use lab_core::modules::{Receiver, Sender, ID};

use super::{PipelineRequest, Runtime, PipelineSetup};

impl Runtime {
    pub fn start_pipelines<'a>(
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

            let labs = self.pipelines.clone();
            let modules = self.modules.clone();
            let mut last_steps_id: ID = 0;

            for key in self.pipelines_keys.iter() {
                let mut pipeline = labs.get(key).unwrap().clone();
                let modules = modules.clone();
                let sender_pipeline = sender_pipeline.clone();
                let sender_control = sender_control.clone();
                let initial_step_id = last_steps_id;
                last_steps_id += pipeline.lab.pipeline.len() as ID;

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
}
