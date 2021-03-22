use super::{tasks::DriverReference, Options};
use crate::libraries::resources::DefaultResourceManager;
use crate::libraries::{helpers::keys, resources::ResourceManagerProvider};
use crate::libraries::{lifecycle::HeartBeat, recording::SequentialWebVTTWriter};
use std::sync::Arc;
use tokio::{fs::File, sync::Mutex};

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub driver_reference: DriverReference,
    pub heart_beat: HeartBeat<Self, DefaultResourceManager>,
    pub id: String,
    pub options: Options,
    pub webvtt: Arc<Mutex<Option<SequentialWebVTTWriter<File>>>>,
}

impl Context {
    pub async fn new(redis_url: String, options: Options) -> Self {
        let id = options.id.clone();
        let heart_beat = HeartBeat::new();

        heart_beat
            .add_beat(&keys::session::heartbeat::node(&id), 60, 120)
            .await;

        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            driver_reference: DriverReference::new(),
            heart_beat,
            id,
            options,
            webvtt: Arc::new(Mutex::new(None)),
        }
    }
}

impl ResourceManagerProvider<DefaultResourceManager> for Context {
    fn resource_manager(&self) -> DefaultResourceManager {
        self.resource_manager.clone()
    }
}
