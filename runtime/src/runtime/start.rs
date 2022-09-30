use std::{collections::HashMap, sync::mpsc};

use lab_core::modules::{Receiver, Sender};

use super::{PipelineRequest, Runtime};

impl Runtime {
    pub fn start(&self) {
        let mut pipeline_steps_ref = HashMap::new();
        let mut pipeline_senders = HashMap::new();
        let (sender_control, receiver_pipelines): (
            Sender<PipelineRequest>,
            Receiver<PipelineRequest>,
        ) = mpsc::channel();

        self.start_pipelines(
            sender_control,
            &mut pipeline_steps_ref,
            &mut pipeline_senders,
        );

        self.listener(receiver_pipelines, pipeline_steps_ref, pipeline_senders);
    }
}
