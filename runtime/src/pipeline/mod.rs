mod data;
mod listener_pipelines;
mod listener_steps;
mod pipeline;

use data::PipelineData;
pub use pipeline::Pipeline;

use lab_core::{
    modules::{BinSender, Response, ID},
};

use core::panic;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use crate::runtime::{Modules, PipelineRequest, PipelineSetup};

impl Pipeline {
    pub fn start(
        &mut self,
        modules: Modules,
        sender_setup_runtime: Sender<PipelineSetup>,
        sender_pipelines: Sender<PipelineRequest>,
        initial_step_id: ID,
    ) -> Result<(), ()> {
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

        let mut pipeline_data =
            PipelineData::new(sender_pipelines.clone(), self.debug_trace.clone());
        let (sender_steps, receiver_steps): (Sender<Response>, Receiver<Response>) =
            mpsc::channel();

        {
            let (sender_bin, receiver_bin): (Sender<BinSender>, Receiver<BinSender>) =
                mpsc::channel();

            self.load_and_start_steps(
                initial_step_id,
                &modules,
                &mut pipeline_data,
                sender_steps.clone(),
                sender_bin.clone(),
            );

            Self::wait_senders(&mut pipeline_data, receiver_bin);

            let pipeline_data_thread = pipeline_data.clone();

            self.listener_steps(
                receiver_steps,
                pipeline_data_thread,
                sender_pipelines,
                initial_step_id,
            );
        }

        self.listener_pipelines(
            receiver_pipelines,
            pipeline_data,
            initial_step_id,
            sender_steps,
        );

        Ok(())
    }
}
