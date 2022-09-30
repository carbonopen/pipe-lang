use crate::{
    lab::Lab,
    trace::{DebugTrace, PipelineTrace},
};
use lab_core::modules::ID;

use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use super::data::PipelineData;

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub id: ID,
    pub key: String,
    pub lab: Lab,
    pub references: HashMap<String, ID>,
    pub pipeline_traces: Arc<Mutex<PipelineTrace>>,
    pub debug_trace: DebugTrace,
    pub pipeline_data: PipelineData,
    pub initial_step_id: ID
}

impl Pipeline {
    pub fn new(
        id: ID,
        key: String,
        lab: Lab,
        pipeline_traces: Arc<Mutex<PipelineTrace>>,
        debug_trace: DebugTrace,
    ) -> Self {
        Self {
            id,
            key,
            lab,
            references: HashMap::default(),
            pipeline_traces,
            pipeline_data: PipelineData::new(debug_trace.clone()),
            debug_trace,
            initial_step_id: 0,
        }
    }

    pub fn add_references(&mut self, references: HashMap<String, ID>) {
        self.references = references;
    }

}
