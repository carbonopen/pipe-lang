use pipe_core::modules::{Request, ID};
use serde_json::Value;

use std::sync::mpsc::Sender;
use std::{collections::HashMap, fmt::Debug};

use crate::{pipe::ModuleType, runtime::PipelineRequest};

#[derive(Debug, Clone)]
pub struct StepConfig {
    pub id: ID,
    pub pipeline_id: ID,
    pub reference: String,
    pub params: HashMap<String, Value>,
    pub producer: bool,
    pub default_attach: Option<String>,
    pub tags: HashMap<String, Value>,
    pub args: HashMap<String, Value>,
}

#[warn(dead_code)]
#[derive(Debug, Clone)]
pub struct Step {
    pub pipeline_id: ID,
    pub module_type: ModuleType,
    pub sender: Option<Sender<Request>>,
    pub config: StepConfig,
    pub sender_pipeline: Option<Sender<PipelineRequest>>,
    pub params: HashMap<String, Value>,
}

impl Step {
    pub fn send(&self, mut request: Request) -> Result<(), ()> {
        match self.module_type {
            ModuleType::Bin => match &self.sender {
                Some(sender) => match sender.send(request) {
                    Ok(_) => Ok(()),
                    Err(_) => Err(()),
                },
                None => Err(()),
            },
            ModuleType::Pipeline => match &self.sender_pipeline {
                Some(sender) => {
                    request.set_and_resolve_args(self.params.clone());

                    match sender.send(PipelineRequest::from_request(
                        request,
                        Some(self.pipeline_id),
                        None,
                        false,
                    )) {
                        Ok(_) => Ok(()),
                        Err(_) => Err(()),
                    }
                }
                None => Err(()),
            },
        }
    }
}
