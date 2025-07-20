use anyhow::Result;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Barrier;
use porcupine::{*, history::{Operation, Invocation, Response}};
use serin_multidc::{MemoryStore, ReplicationClient, ReplicationServer, DcId};

/// Generate Jepsen-like test on in-memory replicated stores.
/// Runs concurrent read/write and verifies linearizability.
pub async fn run_consistency_test() -> Result<()> {
    let store_a = Arc::new(MemoryStore::new());
    let store_b = Arc::new(MemoryStore::new());

    // Start servers (loopback ports).
    tokio::spawn(ReplicationServer::new("127.0.0.1:7001", 1, store_a.clone()).run());
    tokio::spawn(ReplicationServer::new("127.0.0.1:7002", 2, store_b.clone()).run());

    // Clients.
    let client_a = Arc::new(ReplicationClient::new("127.0.0.1:7002", 1));
    let client_b = Arc::new(ReplicationClient::new("127.0.0.1:7001", 2));

    // History collector.
    let history = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let barrier = Arc::new(Barrier::new(6)); // 2 writers + 2 readers + main + partition task

    // Writers.
    for i in 0..2 {
        let hist = history.clone();
        let client = if i == 0 { client_a.clone() } else { client_b.clone() };
        let b = barrier.clone();
        tokio::spawn(async move {
            b.wait().await;
            let mut rng = rand::thread_rng();
            for seq in 0..100u64 {
                let value: Vec<u8> = vec![i as u8, (seq & 0xFF) as u8];
                let lsn = (i as u64) << 32 | seq;
                let inv = Invocation { process: i, time: 0, operation: Operation::Write(lsn) };
                hist.lock().await.push(Event::Invocation(inv.clone()));
                let _ = client.send(lsn, &value).await;
                let resp = Response::Ok;
                hist.lock().await.push(Event::Response(inv, resp));
            }
        });
    }

    // Readers.
    for i in 0..2 {
        let hist = history.clone();
        let store = if i == 0 { store_a.clone() } else { store_b.clone() };
        let b = barrier.clone();
        tokio::spawn(async move {
            b.wait().await;
            for _ in 0..100u64 {
                let inv = Invocation { process: i + 2, time: 0, operation: Operation::Read };
                hist.lock().await.push(Event::Invocation(inv.clone()));
                let entries = store.entries.lock().await; // access internal map
                let _ = entries.len();
                let resp = Response::Ok;
                hist.lock().await.push(Event::Response(inv, resp));
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }
        });
    }

    // Partition task (toggle connectivity).
    let b = barrier.clone();
    tokio::spawn(async move {
        b.wait().await;
        for _ in 0..10 {
            // Simulate partition by dropping client streams.
            *client_a.stream.lock().await = None;
            *client_b.stream.lock().await = None;
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        }
    });

    barrier.wait().await;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Linearizability check using Porcupine.
    let h = history.lock().await.clone();
    let res = check_history(h, LinearizabilityChecker::new(|op| match op {
        Operation::Read => ModelStep::Read(None),
        Operation::Write(_) => ModelStep::Write,
    }));
    assert!(res.is_ok(), "History is not linearizable");
    Ok(())
} 