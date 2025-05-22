use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
struct LogEntry {
    code: u16,
    service: String,
    timestamp: u64,
    message: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Can't listen");
    info!("Aggregator WS listening on {}", addr);

    let peers = Arc::new(Mutex::new(HashSet::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        info!("New TCP connection from {}", addr); // ✅ Log the IP before WebSocket upgrade

        let ws_stream = accept_async(stream).await.expect("Failed to accept");
        let peer_set = peers.clone();

        tokio::spawn(async move {
            let (mut ws_tx, mut ws_rx) = ws_stream.split();
            {
                // Optional: Store `addr` in `peer_set` if you really need to track peers
                peer_set.lock().unwrap().insert(addr.to_string());
            }
            info!("Client WebSocket connected from {}", addr);

            while let Some(msg) = ws_rx.next().await {
                match msg {
                    Ok(Message::Text(txt)) => {
                        if let Ok(log) = serde_json::from_str::<LogEntry>(&txt) {
                            info!("[LOG] {:?}", log);
                            let critical_codes = [400, 401, 404];
                            if critical_codes.contains(&log.code) {
                                notify(&txt).await;
                            }
                        } else {
                            error!("Invalid log JSON: {}", txt);
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => {
                        info!("Client disconnected from {}", addr);
                        break;
                    }
                    _ => {}
                }
            }
        });
    }
}
