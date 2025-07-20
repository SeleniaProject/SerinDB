use anyhow::Result;
use hyper::{service::{make_service_fn, service_fn}, Body, Request, Response, Server, StatusCode};
use prometheus::{Encoder, TextEncoder, IntCounter, Histogram, HistogramOpts};
use std::sync::Arc;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use once_cell::sync::Lazy;

pub static CONNECTIONS_TOTAL: Lazy<IntCounter> = Lazy::new(|| prometheus::register_int_counter!("serin_connections_total", "Total client connections").unwrap());
pub static QUERIES_TOTAL: Lazy<IntCounter> = Lazy::new(|| prometheus::register_int_counter!("serin_queries_total", "Total queries processed").unwrap());
pub static QUERY_LATENCY_SECS: Lazy<Histogram> = Lazy::new(|| {
    let opts = HistogramOpts::new("serin_query_latency_seconds", "Query latency in seconds").buckets(vec![0.0005,0.001,0.005,0.01,0.05,0.1,0.5,1.0]);
    prometheus::register_histogram!(opts).unwrap()
});

/// Launch Prometheus exporter HTTP server on given address.
/// When `basic_auth` is Some((user, pass)), requires Authorization header.
pub async fn serve(addr: &str, basic_auth: Option<(String, String)>) -> Result<()> {
    let make_svc = make_service_fn(move |_| {
        let auth = basic_auth.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| metrics_handler(req, auth.clone())))
        }
    });
    let server = Server::bind(&addr.parse()?).serve(make_svc);
    tokio::spawn(async move { if let Err(e) = server.await { eprintln!("Metrics server error: {e}"); } });
    Ok(())
}

async fn metrics_handler(req: Request<Body>, auth: Option<(String, String)>) -> Result<Response<Body>, hyper::Error> {
    if req.uri().path() != "/metrics" {
        return Ok(Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap());
    }
    if let Some((u, p)) = auth {
        if let Some(header) = req.headers().get("Authorization") {
            let expected = format!("Basic {}", B64.encode(format!("{}:{}", u, p)));
            if header.to_str().unwrap_or("") != expected {
                return Ok(Response::builder().status(StatusCode::UNAUTHORIZED).body(Body::empty()).unwrap());
            }
        } else {
            return Ok(Response::builder().status(StatusCode::UNAUTHORIZED).body(Body::empty()).unwrap());
        }
    }
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    Ok(Response::builder().status(StatusCode::OK).body(Body::from(buffer)).unwrap())
} 