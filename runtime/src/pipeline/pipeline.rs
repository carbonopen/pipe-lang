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

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub id: ID,
    pub key: String,
    pub lab: Lab,
    pub references: HashMap<String, ID>,
    pub pipeline_traces: Arc<Mutex<PipelineTrace>>,
    pub debug_trace: DebugTrace,
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
            debug_trace,
        }
    }

    pub fn add_references(&mut self, references: HashMap<String, ID>) {
        self.references = references;
    }
}
