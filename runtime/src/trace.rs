use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use lab_core::modules::{Request, ID};

use crate::runtime::PipelineRequest;

#[derive(Debug)]
pub struct TracePipelineItem {
    items: HashMap<ID, PipelineRequest>,
    initial: ID,
}

#[derive(Debug)]
pub struct PipelineTrace {
    pub traces: HashMap<ID, TracePipelineItem>,
}

impl PipelineTrace {
    pub fn new() -> Self {
        Self {
            traces: HashMap::new(),
        }
    }

    pub fn remove_trace(&mut self, trace_id: &ID, pipeline_id: &ID) {
        match self.traces.get_mut(trace_id) {
            Some(pipeline_trace) => {
                if pipeline_trace.initial.eq(pipeline_id) {
                    if self.traces.remove(trace_id).is_none() {
                        panic!("Trace was previously removed");
                    }
                } else if pipeline_trace.items.remove(pipeline_id).is_none() {
                    panic!("Trace not found on pipeline");
                }
            }
            None => panic!("Pipeline trace not found"),
        }
    }

    pub fn add_trace(&mut self, pipeline_id: &ID, trace_id: &ID, request: PipelineRequest) {
        match self.traces.get_mut(trace_id) {
            Some(pipeline_trace) => {
                pipeline_trace.items.insert(*pipeline_id, request);
            }
            None => {
                self.traces.insert(
                    *trace_id,
                    TracePipelineItem {
                        items: {
                            let mut map = HashMap::new();
                            map.insert(*pipeline_id, request);
                            map
                        },
                        initial: *pipeline_id,
                    },
                );
            }
        }
    }

    pub fn get_trace(&self, trace_id: &ID,  pipeline_id: &ID) -> Option<&PipelineRequest> {
        match self.traces.get(trace_id) {
            Some(pipeline_trace) => match pipeline_trace.items.get(pipeline_id) {
                Some(request) => Some(request),
                None => None,
            },
            None => None,
        }
    }
}

impl Default for PipelineTrace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct TraceItem {
    trace_id: ID,
    pipeline_id: ID,
    step_id: ID,
    request: Request,
}

impl PartialEq for TraceItem {
    fn eq(&self, other: &Self) -> bool {
        self.trace_id == other.trace_id
            && self.pipeline_id == other.pipeline_id
            && self.step_id == other.step_id
    }
}

#[derive(Debug)]
pub struct Tracer {
    traces: VecDeque<TraceItem>,
    trace_by_trace_id: HashMap<ID, Vec<TraceItem>>,
    size: usize,
}

impl Tracer {
    pub fn new(size: usize) -> Self {
        Self {
            traces: VecDeque::new(),
            trace_by_trace_id: HashMap::new(),
            size,
        }
    }

    pub fn add(&mut self, trace_id: ID, pipeline_id: ID, step_id: ID, request: Request) {
        let trace_item = TraceItem {
            trace_id,
            pipeline_id,
            step_id,
            request,
        };

        self.traces.push_back(trace_item.clone());

        println!("tracing: origin {} -> step_id {}, pipeline_id {}, trace_id {}", trace_item.request.origin, trace_item.step_id, trace_item.pipeline_id, trace_item.trace_id);

        match self.trace_by_trace_id.get_mut(&trace_id) {
            Some(trace) => {
                trace.push(trace_item);
            }
            None => {
                self.trace_by_trace_id.insert(trace_id, vec![trace_item]);
            }
        };

        if self.traces.len().eq(&self.size) {
            let removed = self.traces.pop_front().unwrap();

            let trace = self.trace_by_trace_id.get_mut(&removed.trace_id).unwrap();

            if let Some(index) = trace.iter().position(|item| removed.eq(item)) {
                trace.remove(index);
                if trace.is_empty() {
                    self.trace_by_trace_id.remove(&removed.trace_id);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DebugTrace {
    tracer: Arc<Mutex<Tracer>>,
    enabled: bool,
}

impl DebugTrace {
    pub fn new(size: usize, enabled: bool) -> Self {
        Self {
            tracer: Arc::new(Mutex::new(Tracer::new(size))),
            enabled,
        }
    }

    pub fn add(&self, trace_id: ID, pipeline_id: ID, step_id: ID, request: Request) {
        self.tracer
            .lock()
            .unwrap()
            .add(trace_id, pipeline_id, step_id, request)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
