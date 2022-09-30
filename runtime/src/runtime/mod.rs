mod builder;
mod listener;
mod start;
mod start_pipeline;
#[cfg(test)]
mod tests;
use crate::lab::ModuleType;
use crate::pipeline::Pipeline;
use lab_core::modules::{Request, ID};
use std::sync::mpsc::Sender;
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, Clone)]
pub struct ModuleInner {
    pub module_type: ModuleType,
    pub key: String,
}

pub type Alias = HashMap<String, ModuleInner>;
pub type Pipelines = HashMap<String, Pipeline>;
pub type Aliases = HashMap<String, Alias>;
pub type Bins = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct Modules {
    pub bins: Bins,
    pub aliases: Aliases,
}

impl Modules {
    pub(crate) fn get(&self, owner: &str, alias: &str) -> ModuleInner {
        self.aliases.get(owner).unwrap().get(alias).unwrap().clone()
    }

    pub(crate) fn get_bin_key(&self, key: &str) -> String {
        self.bins.get(key).unwrap().clone()
    }
}

#[derive(Debug)]
pub struct PipelineTarget {
    pub id: ID,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct PipelineRequest {
    pub step_attach: Option<ID>,
    pub pipeline_attach: Option<ID>,
    pub request: Request,
    pub return_pipeline: bool,
    pub return_to: (ID, ID),
}

impl PipelineRequest {
    pub fn from_request(
        request: Request,
        pipeline_attach: Option<ID>,
        step_attach: Option<ID>,
        return_pipeline: bool,
    ) -> Self {
        Self {
            request: Request {
                payload: request.payload,
                origin: request.origin,
                trace: request.trace,
                steps: request.steps.clone(),
            },
            step_attach,
            pipeline_attach,
            return_pipeline,
            return_to: (0, 0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineSetup {
    pub tx: Sender<PipelineRequest>,
    pub id: ID,
}

#[derive(Debug)]
pub struct Runtime {
    pipelines: Pipelines,
    pipelines_keys: Vec<String>,
    modules: Modules,
    references: HashMap<String, ID>,
}
