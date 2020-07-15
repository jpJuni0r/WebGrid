use log::{info, trace};
use redis::{aio::ConnectionLike, AsyncCommands};
use std::fmt;

// Various timeouts in seconds
#[derive(Debug)]
pub enum Timeout {
    Queue,
    Scheduling,
    NodeStartup,
    DriverStartup,
    SessionTermination,
    SlotReclaimInterval,
}

impl fmt::Display for Timeout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Timeout {
    fn default(&self) -> usize {
        match *self {
            // Manager
            Timeout::Queue => 120,
            Timeout::Scheduling => 60,
            Timeout::NodeStartup => 45,
            // Node
            Timeout::DriverStartup => 30,
            Timeout::SessionTermination => 60,
            // Orchestrator
            Timeout::SlotReclaimInterval => 300,
        }
    }

    pub async fn get<C: ConnectionLike + AsyncCommands>(&self, con: &mut C) -> usize {
        let key = format!("{}", self).to_lowercase();

        trace!("Reading timeout {}", key);
        let timeout: Option<usize> = con.hget("timeouts", &key).await.ok();

        match timeout {
            Some(timeout) => timeout,
            None => {
                info!("Initializing timeout {} to default value", key);
                let default = self.default();
                let _: Option<()> = con.hset("timeouts", key, default).await.ok();
                default
            }
        }
    }
}