use super::{PipelineRequest, Runtime};
use lab_core::modules::{Receiver, Sender};
use std::collections::HashMap;

impl Runtime {
    pub fn listener(
        &self,
        receiver_pipelines: Receiver<PipelineRequest>,
        pipeline_steps_ref: HashMap<u32, u32>,
        pipeline_senders: HashMap<u32, Sender<PipelineRequest>>,
    ) {
        for pipeline_request in receiver_pipelines {
            let pipeline_id = match pipeline_request.pipeline_attach {
                Some(id) => id,
                None => pipeline_steps_ref
                    .get(&pipeline_request.step_attach.unwrap())
                    .unwrap()
                    .clone(),
            };

            let sender = pipeline_senders
                .get(&pipeline_id)
                .expect("Pipeline sender not found");

            match sender.send(pipeline_request) {
                Ok(_) => continue,
                Err(err) => panic!("{:?}", err),
            }
        }
    }
}
