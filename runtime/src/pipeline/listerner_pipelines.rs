use lab_core::modules::Response;
use crate::runtime::PipelineRequest;
use super::{data::PipelineData, Pipeline};
use lab_core::modules::ID;
use core::panic;
use std::sync::mpsc::{Receiver, Sender};

impl Pipeline {
    pub fn listener_pipelines(
        &mut self,
        receiver_pipelines: Receiver<PipelineRequest>,
        pipeline_data: PipelineData,
        initial_step_id: ID,
        sender_steps: Sender<Response>,
    ) {
        for pipeline_request in receiver_pipelines {
            let step_id = match pipeline_request.step_attach {
                Some(step_attach) if pipeline_request.return_pipeline == true => {
                    let origin = step_attach + 1;
                    let attach = pipeline_data
                        .steps
                        .get(&origin)
                        .unwrap()
                        .config
                        .default_attach
                        .clone();

                    let response = Response {
                        payload: pipeline_request.request.payload,
                        attach,
                        origin,
                        trace: pipeline_request.request.trace,
                    };

                    match sender_steps.send(response) {
                        Ok(_) => continue,
                        Err(_) => panic!("Return error"),
                    }
                }
                Some(step_attach) => step_attach,
                None => initial_step_id,
            };
            let step = match pipeline_data.steps.get(&step_id) {
                Some(step) => step,
                None => panic!("Step not found"),
            };
            let trace_id = pipeline_request.request.trace.trace_id;

            match step.send(pipeline_request.request.clone()) {
                Ok(_) if pipeline_request.return_pipeline == false => {
                    self.pipeline_traces.lock().unwrap().add_trace(
                        &self.id,
                        &trace_id,
                        pipeline_request,
                    );
                }
                Err(_) => {
                    panic!("trace_id: {} |  Sender by step id 0 not exist", trace_id);
                }
                _ => (),
            }
        }
    }
}
