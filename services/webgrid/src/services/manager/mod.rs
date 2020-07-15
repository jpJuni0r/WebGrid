use helpers::{constants, SharedOptions};
use lifecycle::Heart;
use log::info;
use scheduling::{schedule, JobScheduler, StatusServer};
use structopt::StructOpt;

mod context;
mod jobs;
mod structures;
mod tasks;

use context::Context;
use jobs::{RegistrationJob, SessionHandlerJob};
pub use structures::*;

#[derive(Debug, StructOpt)]
/// Endpoint for handling session creation
///
/// Handles scheduling and provisioning lifecycle of sessions.
pub struct Options {
    /// Unique instance identifier
    #[structopt(env)]
    id: String,

    /// Host under which the manager is reachable by other services
    #[structopt(env = "MANAGER_HOST")]
    host: String,

    /// Port on which the HTTP server will listen
    #[structopt(short, long, default_value = constants::PORT_MANAGER)]
    port: u16,
}

pub async fn run(shared_options: SharedOptions, options: Options) {
    let (mut heart, _) = Heart::new();

    let context = Context::new(shared_options.redis);
    let scheduler = JobScheduler::new();

    context.spawn_heart_beat(&options.id, &scheduler).await;

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let session_handler_job = SessionHandlerJob::new(options.port);
    let registration_job = RegistrationJob::new(options.id, options.host, options.port);

    schedule!(scheduler, context, {
        status_job,
        session_handler_job
        registration_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;
}