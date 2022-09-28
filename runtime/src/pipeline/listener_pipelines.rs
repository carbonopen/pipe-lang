use super::Pipeline;
use crate::runtime::PipelineRequest;
use core::panic;
use lab_core::modules::Response;
use lab_core::modules::ID;
use std::sync::mpsc::{Receiver, Sender};

impl Pipeline {
    fn send_step(
        &self,
        step_attach: ID,
        pipeline_request: PipelineRequest,
        sender_steps: Sender<Response>,
    ) -> Result<(), std::sync::mpsc::SendError<Response>> {
        let origin = step_attach + 1;
        let attach = self
            .pipeline_data
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

        sender_steps.send(response)
    }

    pub fn listener_pipelines(
        &mut self,
        receiver_pipelines: Receiver<PipelineRequest>,
        initial_step_id: ID,
        sender_steps: Sender<Response>,
    ) {
        for pipeline_request in receiver_pipelines {
            let step_id = match pipeline_request.step_attach {
                Some(step_attach) if pipeline_request.return_pipeline == true => {
                    match self.send_step(step_attach, pipeline_request, sender_steps.clone()) {
                        Ok(_) => continue,
                        Err(_) => panic!("Return error"),
                    }
                }
                Some(step_attach) => step_attach,
                None => initial_step_id,
            };
            let step = match self.pipeline_data.steps.get(&step_id) {
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
