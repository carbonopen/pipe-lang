use super::pipeline::Pipeline;
use crate::runtime::PipelineRequest;
use core::panic;
use lab_core::modules::Response;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

impl Pipeline {
    pub fn listener_steps<'a>(
        &self,
        receiver_steps: Receiver<Response>,
        sender_pipelines: Sender<PipelineRequest>,
    ) {
        let pipeline_id = self.id;
        let pipeline_traces = self.pipeline_traces.clone();
        let pipeline_data = self.pipeline_data.clone();
        let initial_step_id = self.initial_step_id.clone();

        thread::spawn(move || {
            for response in receiver_steps {
                let request = pipeline_data.get_request(response.clone());

                if let Some(attach) = response.attach {
                    match pipeline_data.get_by_reference(&attach) {
                        Some(step) => match step.send(request) {
                            Ok(_) => continue,
                            Err(err) => {
                                panic!("{}", err);
                            }
                        },
                        None => {
                            panic!("Reference {} not found", attach);
                        }
                    };
                } else {
                    let next_step = response.origin + 1;

                    match pipeline_data.steps.get(&next_step) {
                        Some(step) => match step.send(request) {
                            Ok(_) => continue,
                            Err(err) => {
                                panic!("{}", err);
                            }
                        },
                        None => {
                            let mut lock_pipeline_traces = pipeline_traces.lock().unwrap();

                            match lock_pipeline_traces
                                .get_trace(&pipeline_id, &response.trace.trace_id)
                            {
                                Some(pipeline_request) => {
                                    match sender_pipelines.send(PipelineRequest::from_request(
                                        request,
                                        None,
                                        Some(pipeline_request.request.origin),
                                        true,
                                    )) {
                                        Ok(_) => {
                                            lock_pipeline_traces.remove_trace(
                                                &pipeline_id,
                                                &response.trace.trace_id,
                                            );
                                        }
                                        Err(err) => {
                                            panic!("{:#?}", err);
                                        }
                                    };
                                }
                                None => {
                                    match &pipeline_data.steps.get(&initial_step_id) {
                                        Some(step) => match step.send(request) {
                                            Ok(_) => continue,
                                            Err(err) => {
                                                panic!("{}", err);
                                            }
                                        },
                                        None => {
                                            panic!(
                                                "trace_id: {} |  Sender by step id {} not exist",
                                                response.trace.trace_id, next_step
                                            );
                                        }
                                    };
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}
