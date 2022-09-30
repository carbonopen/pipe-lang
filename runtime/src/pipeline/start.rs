use lab_core::modules::{BinSender, Response, ID};

use core::panic;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use crate::runtime::{Modules, PipelineRequest, PipelineSetup};

use super::Pipeline;

impl Pipeline {
    pub fn start(
        &mut self,
        modules: Modules,
        sender_setup_runtime: Sender<PipelineSetup>,
        sender_pipelines: Sender<PipelineRequest>,
        initial_step_id: ID,
    ) -> Result<(), ()> {
        self.initial_step_id = initial_step_id;

        let (sender_request_pipeline, receiver_pipelines): (
            Sender<PipelineRequest>,
            Receiver<PipelineRequest>,
        ) = mpsc::channel();

        if sender_setup_runtime
            .send(PipelineSetup {
                tx: sender_request_pipeline.clone(),
                id: self.id,
            })
            .is_err()
        {
            panic!("An error occurred while starting the pipeline.");
        }

        drop(sender_setup_runtime);

        self.pipeline_data
            .set_pipeline_sender(sender_pipelines.clone());
        let (sender_steps, receiver_steps): (Sender<Response>, Receiver<Response>) =
            mpsc::channel();

        {
            let (sender_bin, receiver_bin): (Sender<BinSender>, Receiver<BinSender>) =
                mpsc::channel();

            self.load_and_start_steps(
                &modules,
                sender_steps.clone(),
                sender_bin.clone(),
                receiver_bin,
            );

            self.listener_steps(receiver_steps, sender_pipelines);
        }

        self.listener_pipelines(receiver_pipelines, sender_steps);

        Ok(())
    }
}
