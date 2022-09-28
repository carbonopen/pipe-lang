use lab_core::modules::{History, Request, Response, ID};

use std::sync::mpsc::Sender;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use crate::{lab::ModuleType, runtime::PipelineRequest, trace::DebugTrace};

use super::step::{Step, StepConfig};
#[derive(Debug, Clone)]
pub struct PipelineData {
    pub steps: HashMap<u32, Step>,
    pub history: Arc<Mutex<History>>,
    pub reference: HashMap<String, u32>,
    pub total_bins: u32,
    pub pipeline_sender: Option<Sender<PipelineRequest>>,
    pub debug_trace: DebugTrace,
}

impl PipelineData {
    pub fn new(debug_trace: DebugTrace) -> Self {
        Self {
            steps: HashMap::default(),
            history: Arc::new(Mutex::new(History::new())),
            total_bins: 0,
            reference: HashMap::default(),
            pipeline_sender: None,
            debug_trace,
        }
    }

    pub fn set_pipeline_sender(&mut self, pipeline_sender: Sender<PipelineRequest>){
        self.pipeline_sender = Some(pipeline_sender);
    }

    pub fn insert_pipeline(&mut self, id: ID, pipeline_id: ID, config: StepConfig) {
        self.reference.insert(config.reference.clone(), id);

        self.steps.insert(
            id,
            Step {
                pipeline_id,
                module_type: ModuleType::Pipeline,
                sender: None,
                params: config.params.clone(),
                config,
                sender_pipeline: self.pipeline_sender.clone(),
                debug_trace: self.debug_trace.clone(),
            },
        );
    }

    pub fn insert_bin(&mut self, id: ID, pipeline_id: ID, config: StepConfig) {
        self.reference.insert(config.reference.clone(), id);

        self.steps.insert(
            id,
            Step {
                pipeline_id,
                module_type: ModuleType::Bin,
                sender: None,
                params: config.params.clone(),
                config,
                sender_pipeline: None,
                debug_trace: self.debug_trace.clone(),
            },
        );
        self.total_bins += 1;
    }

    pub fn bin_sender(&mut self, id: ID, sender: Sender<Request>) {
        match self.steps.get_mut(&id) {
            Some(step) => step.sender = Some(sender),
            None => (),
        }
    }

    pub fn update_history(
        &self,
        response: &Response,
    ) -> Option<HashMap<String, lab_core::modules::Step>> {
        match self.steps.get(&response.origin) {
            Some(step) => {
                let mut his_lock = self.history.lock().unwrap();

                his_lock.insert(
                    response.trace.clone(),
                    step.config.reference.clone(),
                    response.clone(),
                );

                match his_lock.steps.get(&response.trace.trace_id) {
                    Some(steps) => Some(steps.clone()),
                    None => None,
                }
            }
            None => None,
        }
    }

    pub fn get_request(&self, response: Response) -> Request {
        let steps = self.update_history(&response);
        Request {
            origin: response.origin,
            payload: response.payload,
            steps,
            trace: response.trace,
        }
    }

    pub fn get_by_reference(&self, reference: &str) -> Option<&Step> {
        match self.reference.get(reference) {
            Some(id) => self.steps.get(id),
            None => None,
        }
    }
}
