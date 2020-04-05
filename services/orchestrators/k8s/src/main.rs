mod provisioner;

use crate::provisioner::K8sProvisioner;
use orchestrator_core::{start, provisioner::Type};

#[tokio::main]
async fn main() {
    let provisioner = K8sProvisioner::new().await;
    start(Type::K8s, provisioner).await;
}
