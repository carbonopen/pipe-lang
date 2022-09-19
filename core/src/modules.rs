extern crate serde_json;

use serde::Serialize;
pub use serde_json::json;
use serde_json::{Map, Value};

use std::collections::HashMap;
use std::fmt::Debug;
pub use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::{any::Any, sync::Arc};

#[derive(Debug, Clone, Serialize, Default)]
#[allow(dead_code)]
pub struct Trace {
    pub trace_id: ID,
    pub args: Args,
}

impl Trace {
    pub fn new(trace_id: ID, args: Args) -> Self {
        Self { trace_id, args }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub reference: String,
    pub params: HashMap<String, Value>,
    pub producer: bool,
    pub default_attach: Option<String>,
    pub tags: HashMap<String, Value>,
    pub args: Args,
}

pub type Payload = Result<Option<Value>, Option<Value>>;
pub type Args = HashMap<String, Value>;

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct Step {
    pub origin: ID,
    pub payload: Option<Value>,
    pub trace: Trace,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct History {
    pub steps: HashMap<ID, HashMap<String, Step>>,
}

impl History {
    pub fn new() -> Self {
        Self {
            steps: HashMap::new(),
        }
    }

    pub fn insert(&mut self, trace: Trace, module_name: String, response: Response) {
        let content = Step {
            origin: response.origin,
            payload: response.payload.unwrap(),
            trace: trace.clone(),
        };

        if let Some(step) = self.steps.get_mut(&trace.trace_id) {
            step.insert(module_name, content);
            return;
        };

        let mut step = HashMap::new();
        step.insert(module_name, content);

        self.steps.insert(trace.trace_id, step);
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Request {
    pub origin: ID,
    pub payload: Payload,
    pub steps: Option<HashMap<String, Step>>,
    pub trace: Trace,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            origin: Default::default(),
            payload: Ok(None),
            steps: Default::default(),
            trace: Default::default(),
        }
    }
}

impl Request {
    pub fn from_payload(payload: Value) -> Self {
        Self {
            payload: Ok(Some(payload)),
            ..Default::default()
        }
    }

    pub fn set_args(&mut self, args: Args) {
        self.trace.args = args;
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Response {
    pub payload: Result<Option<Value>, Option<Value>>,
    pub attach: Option<String>,
    pub origin: ID,
    pub trace: Trace,
}

pub type Listener = Receiver<Request>;
pub type Speaker = Sender<Response>;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Return {
    pub payload: Result<Option<Value>, Option<Value>>,
    pub attach: Option<String>,
    pub trace: Trace,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct BinSender {
    pub tx: Sender<Request>,
    pub id: ID,
}

#[derive(Debug)]
pub struct ResponseComplete {
    pub origin: String,
    pub payload: Result<Option<Value>, Option<Value>>,
    pub origin_params: Option<Value>,
}

pub type ID = u32;

#[allow(dead_code)]
pub trait Module: Any + Send {
    fn requests(&self, id: ID, request: Sender<BinSender>) -> Listener {
        let (tx_req, rx_req): (Sender<Request>, Listener) = channel();
        request.send(BinSender { tx: tx_req, id }).unwrap();
        rx_req
    }

    fn start(
        &self,
        _id: ID,
        _request: Sender<BinSender>,
        _response: Sender<Response>,
        _config: Config,
    ) {
    }
}

impl Debug for dyn Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module").finish()
    }
}

use uuid::Uuid;

pub fn get_trace() -> String {
    Uuid::new_v4().to_string()
}

pub struct TraceId {
    pub id: ID,
}

impl TraceId {
    pub fn new() -> TraceId {
        TraceId {
            id: ID::min_value(),
        }
    }

    pub fn global() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(TraceId::new()))
    }

    pub fn get_trace(&mut self) -> ID {
        self.id = self.id + 1;

        if self.id > ID::max_value() {
            self.id = ID::min_value();
        }

        self.id.clone()
    }
}

#[macro_export]
macro_rules! declare_module {
    ($module_type:ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn _Module() -> *mut $crate::modules::Module {
            let constructor: fn() -> $module_type = $constructor;
            let object = constructor();
            let boxed: Box<$crate::modules::Module> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}

#[macro_export]
macro_rules! create_module_raw {
    ($handler:ident) => {
        #[derive(Debug, Default, Clone)]
        pub struct Custom {}

        impl $crate::modules::Module for Custom {
            fn start(
                &self,
                module_id: $crate::modules::ID,
                req: $crate::modules::Sender<$crate::modules::BinSender>,
                res: $crate::modules::Sender<$crate::modules::Response>,
                config: $crate::modules::Config,
            ) {
                $handler(module_id, self.requests(module_id, req), res, config)
            }
        }

        declare_module!(Custom, Custom::default);
    };
}

#[macro_export]
macro_rules! create_module_producer {
    ($handler:ident) => {
        #[derive(Debug, Default)]
        pub struct Custom {}

        impl $crate::modules::Module for Custom {
            fn start(
                &self,
                module_id: $crate::modules::ID,
                req: $crate::modules::Sender<$crate::modules::BinSender>,
                res: $crate::modules::Sender<$crate::modules::Response>,
                config: $crate::modules::Config,
            ) {
                let trace =
                    std::sync::Arc::new(std::sync::Mutex::new($crate::modules::TraceId::new()));

                $handler(
                    self.requests(module_id, req),
                    |result: $crate::modules::Return| {
                        res.send($crate::modules::Response {
                            payload: result.payload,
                            attach: result.attach,
                            origin: module_id,
                            trace: $crate::modules::Trace::new(
                                trace.lock().unwrap().get_trace(),
                                result.args,
                            ),
                        })
                        .unwrap();
                    },
                    config,
                )
            }
        }

        declare_module!(Custom, Custom::default);
    };
}

#[macro_export]
macro_rules! create_module {
    ($handler:ident) => {
        #[derive(Debug, Default, Clone)]
        pub struct Custom {}

        impl $crate::modules::Module for Custom {
            fn start(
                &self,
                module_id: $crate::modules::ID,
                req: $crate::modules::Sender<$crate::modules::BinSender>,
                res: $crate::modules::Sender<$crate::modules::Response>,
                config: $crate::modules::Config,
            ) {
                $handler(
                    self.requests(module_id, req),
                    |result: $crate::modules::Return| {
                        res.send($crate::modules::Response {
                            payload: result.payload,
                            attach: result.attach,
                            origin: module_id,
                            trace: result.trace,
                        })
                        .unwrap();
                    },
                    config,
                )
            }
        }

        declare_module!(Custom, Custom::default);
    };
}

#[macro_export]
macro_rules! create_module_listener {
    ($handler:ident) => {
        #[derive(Debug, Default)]
        pub struct Custom {}

        impl $crate::modules::Module for Custom {
            fn start(
                &self,
                module_id: $crate::modules::ID,
                req: $crate::modules::Sender<$crate::modules::BinSender>,
                res: $crate::modules::Sender<$crate::modules::Response>,
                config: $crate::modules::Config,
            ) {
                for request in self.requests(module_id, req) {
                    let result = $handler(request);

                    res.send($crate::modules::Response {
                        payload: result.payload,
                        attach: result.attach,
                        origin: module_id,
                        trace: result.trace,
                    })
                    .unwrap();
                }
            }
        }

        declare_module!(Custom, Custom::default);
    };
}

#[macro_export]
macro_rules! create_module_assert_eq {
    ($module:expr, $config:expr) => {
        create_module_assert_eq!($module, $config, Ok(None), Ok(None), true);
    };
    ($module:expr, $config:expr, $payload:expr, $compare:expr) => {
        create_module_assert_eq!($module, $config, $payload, $compare, true);
    };
    ($module:expr, $config:expr, $payload:expr, $compare:expr, $producer:expr) => {
        let (tx_res, rx_res): (
            $crate::modules::Sender<$crate::modules::Response>,
            $crate::modules::Receiver<$crate::modules::Response>,
        ) = $crate::modules::channel();
        let (tx_req, rx_req): (
            $crate::modules::Sender<$crate::modules::Request>,
            $crate::modules::Listener,
        ) = $crate::modules::channel();

        std::thread::spawn(move || {
            $module(
                rx_req,
                |result: $crate::modules::Return| {
                    tx_res
                        .send($crate::modules::Response {
                            payload: result.payload,
                            attach: result.attach,
                            origin: 0,
                            trace: result.trace,
                        })
                        .unwrap();
                },
                $config,
            );
        });

        if ($producer) {
            tx_req
                .send($crate::modules::Request {
                    payload: $payload,
                    origin: 0,
                    steps: None,
                    trace: Default::default(),
                })
                .unwrap();
        }

        let left = rx_res.recv().unwrap().payload;

        assert_eq!(left, $compare)
    };
}

#[macro_export]
macro_rules! create_module_assert_eq_attach {
    ($module:expr, $config:expr, $payload:expr, $compare:expr) => {
        let (tx_res, rx_res): (
            $crate::modules::Sender<$crate::modules::Response>,
            $crate::modules::Receiver<$crate::modules::Response>,
        ) = $crate::modules::channel();
        let (tx_req, rx_req): (
            $crate::modules::Sender<$crate::modules::Request>,
            $crate::modules::Listener,
        ) = $crate::modules::channel();

        std::thread::spawn(move || {
            $module(
                rx_req,
                |result: $crate::modules::Return| {
                    tx_res
                        .send($crate::modules::Response {
                            payload: result.payload,
                            attach: result.attach,
                            origin: 0,
                            trace: result.trace,
                        })
                        .unwrap();
                },
                $config,
            );
        });

        tx_req
            .send($crate::modules::Request {
                payload: $payload,
                origin: 0,
                trace: Default::default(),
                steps: None,
            })
            .unwrap();

        let left = rx_res.recv().unwrap().attach;

        assert_eq!(left, $compare)
    };
}

#[macro_export]
macro_rules! run_module_raw {
    ($module:expr, $config:expr, $tx:ident, $rx:ident) => {
        let ($tx, rreq): (
            $crate::modules::Sender<$crate::modules::Request>,
            $crate::modules::Listener,
        ) = $crate::modules::channel();
        let (tres, $rx): (
            $crate::modules::Sender<$crate::modules::Response>,
            $crate::modules::Receiver<$crate::modules::Response>,
        ) = $crate::modules::channel();

        std::thread::spawn(move || {
            $module(0, rreq, tres, $config);
        });
    };
}
