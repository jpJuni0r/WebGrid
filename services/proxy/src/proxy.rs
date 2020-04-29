use std::convert::Infallible;

use hyper::service::{make_service_fn, service_fn};
use hyper::{
    body::HttpBody, client::HttpConnector, Body, Client, Method, Request, Response, Server,
    StatusCode,
};
use log::{debug, info};
use regex::Regex;
use tokio::sync::mpsc::UnboundedSender;

use crate::watcher::RoutingInfo;
use shared::metrics::MetricsEntry;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type ProxyResult<T> = std::result::Result<T, GenericError>;

static NOTFOUND: &[u8] = b"Not Found";
static NOGATEWAY: &[u8] = b"No managers available to handle the request";
static ALIVE: &[u8] = b"I'm alive";

lazy_static! {
    static ref REGEX_SESSION_PATH: Regex = Regex::new(r"/session/(?P<sid>[^/]*)").unwrap();
}

#[derive(Clone)]
pub struct ProxyServer {
    info: RoutingInfo,
    client: Client<HttpConnector>,
    metrics_tx: UnboundedSender<MetricsEntry>,
}

impl ProxyServer {
    pub fn new(info: RoutingInfo, metrics_tx: UnboundedSender<MetricsEntry>) -> Self {
        ProxyServer {
            info,
            client: Client::new(),
            metrics_tx,
        }
    }

    async fn forward(
        &self,
        mut req: Request<Body>,
        upstream: String,
    ) -> ProxyResult<Response<Body>> {
        let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");

        debug!("{} {} -> {}", req.method(), path, upstream);

        *req.uri_mut() = format!("http://{}{}", upstream, path).parse().unwrap();
        Ok(self.client.request(req).await?)
    }

    async fn handle_session_request(
        &self,
        req: Request<Body>,
        session_id: &str,
    ) -> ProxyResult<Response<Body>> {
        match self.info.get_session_upstream(session_id) {
            Some(upstream) => self.forward(req, upstream).await,
            None => {
                let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
                debug!("{} {} -> BAD GATEWAY (session request)", req.method(), path);

                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(NOTFOUND.into())
                    .unwrap())
            }
        }
    }

    async fn handle_manager_request(&self, req: Request<Body>) -> ProxyResult<Response<Body>> {
        match self.info.get_manager_upstream() {
            Some(upstream) => self.forward(req, upstream).await,
            None => {
                let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
                debug!("{} {} -> BAD GATEWAY (manager request)", req.method(), path);

                Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(NOGATEWAY.into())
                    .unwrap())
            }
        }
    }

    async fn handle(&self, req: Request<Body>) -> ProxyResult<Response<Body>> {
        let req_method = req.method().clone();
        let req_size = req.body().size_hint().lower();
        self.metrics_tx
            .send(MetricsEntry::IncomingTraffic(req_size))
            .ok();

        let path = req
            .uri()
            .path_and_query()
            .map(|x| x.to_string())
            .unwrap_or_else(|| "".to_string());

        let result = if path == "/status" {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(ALIVE.into())
                .unwrap())
        } else if req.method() == Method::POST && path == "/session" {
            self.handle_manager_request(req).await
        } else {
            match REGEX_SESSION_PATH.captures(&path) {
                Some(caps) => self.handle_session_request(req, &caps["sid"]).await,
                None => {
                    let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
                    debug!("{} {} -> NOT FOUND", req.method(), path);

                    Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(NOTFOUND.into())
                        .unwrap())
                }
            }
        };

        if let Ok(response) = &result {
            let status_code = response.status();
            let res_size = response.body().size_hint().lower();

            self.metrics_tx
                .send(MetricsEntry::OutgoingTraffic(res_size))
                .ok();
            self.metrics_tx
                .send(MetricsEntry::RequestProcessed(req_method, status_code))
                .ok();
        }

        result
    }

    pub async fn serve(&self) {
        let make_svc = make_service_fn(|_conn| {
            let p = self.clone();
            async {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let p = p.clone();
                    async move { p.handle(req).await }
                }))
            }
        });

        let addr = ([0, 0, 0, 0], 8080).into();
        let server = Server::bind(&addr).serve(make_svc);

        info!("Listening on {}", addr);
        server.await.unwrap();
    }
}
