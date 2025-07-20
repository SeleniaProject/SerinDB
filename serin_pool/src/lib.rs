//! Simple in-memory connection pool for SerinDB PgWire connections.
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Semaphore};
use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};

pub struct PoolConfig {
    pub max_idle: usize,
    pub max_active: usize,
}

pub struct PooledConn {
    stream: TcpStream,
    created_at: Instant,
}

pub struct ConnectionPool {
    config: PoolConfig,
    idle: Mutex<VecDeque<PooledConn>>,
    sem: Semaphore,
}

impl ConnectionPool {
    pub fn new(config: PoolConfig) -> Self {
        Self { idle: Mutex::new(VecDeque::new()), sem: Semaphore::new(config.max_active), config }
    }

    pub async fn get(&self, addr: &str) -> tokio::io::Result<PooledConn> {
        if let Ok(mut idle) = self.idle.try_lock() {
            if let Some(conn) = idle.pop_front() {
                return Ok(conn);
            }
        }
        let _permit = self.sem.acquire().await.unwrap();
        let stream = TcpStream::connect(addr).await?;
        Ok(PooledConn { stream, created_at: Instant::now() })
    }

    pub async fn release(&self, mut conn: PooledConn) {
        if self.idle.lock().await.len() >= self.config.max_idle {
            let _ = conn.stream.shutdown().await;
            return;
        }
        self.idle.lock().await.push_back(conn);
    }

    pub async fn start_readyz(&self, listen: SocketAddr) {
        let make_svc = make_service_fn(|_|
            async { Ok::<_, hyper::Error>(service_fn(|_req: Request<Body>| async {
                Ok::<_, hyper::Error>(Response::new(Body::from("OK")))
            })) });
        let server = Server::bind(&listen).serve(make_svc);
        tokio::spawn(async move {
            if let Err(e) = server.await { eprintln!("readyz server error: {e}"); }
        });
    }
} 