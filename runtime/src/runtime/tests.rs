use crate::envs::Envs;

use super::*;

#[test]
fn runtime_tet() {
    let envs = Envs::builder();
    match Runtime::builder("example/modules/main.lab", &envs) {
        Ok(run) => run.start(),
        Err(_) => assert!(false),
    }
}
