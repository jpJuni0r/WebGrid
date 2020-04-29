#[macro_use]
extern crate lazy_static;

use redis::Client;
use std::thread;

use shared::{service_init, metrics::MetricsProcessor};

mod config;
mod proxy;
mod watcher;

use crate::config::Config;
use crate::proxy::ProxyServer;
use crate::watcher::RoutingInfo;

#[tokio::main]
async fn main() {
    service_init();

    let config = Config::new().unwrap();

    let client = Client::open(config.clone().redis_url).unwrap();
    let con = client.get_multiplexed_tokio_connection().await.unwrap();

    let info = RoutingInfo::new();
    let proxy = ProxyServer::new(info.clone());

    thread::spawn(move || {
        watcher::main_loop(info, config).unwrap();
    });

    proxy.serve().await;
}
