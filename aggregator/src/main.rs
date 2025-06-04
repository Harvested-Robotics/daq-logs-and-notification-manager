// use chrono::{NaiveDateTime, TimeZone, Utc};
use futures::{SinkExt, StreamExt};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let addr: &'static str = "127.0.0.1:8080";
    let listener: TcpListener = TcpListener::bind(addr).await.expect("Can't listen");
    info!("Aggregator WS listening on {}", addr);

    let peers: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        info!("New TCP connection from {}", addr);

        let ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream> = accept_async(stream).await.expect("Failed to accept");
        let peer_set = peers.clone();

        tokio::spawn(async move {
            let (mut ws_tx, mut ws_rx) = ws_stream.split();
            {
                peer_set.lock().unwrap().insert(addr.to_string());
            }

            info!("Client WebSocket connected from {}", addr);

            while let Some(msg) = ws_rx.next().await {
                match msg {
                    Ok(Message::Text(txt)) => {
                        info!("Received raw message: {}", txt);

                        if let Some(code) = try_extract_code(&txt) {
                            info!("Detected code: {}", code);
                            let critical_codes = [400, 401, 402, 403, 404];
                            if critical_codes.contains(&code) {
                                notify(&txt).await;
                            }
                        } else {
                            info!("No actionable code found in message.");
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

///! This example assumes it may appear at the start or within the messagee.
fn try_extract_code(text: &str) -> Option<u16> {
    for word in text.split(|c: char| !c.is_numeric()) {
        if let Ok(code) = word.parse::<u16>() {
            return Some(code);
        }
    }
    None
}

async fn notify(payload: &str) {
    if let Ok((ws_stream, _)) = tokio_tungstenite::connect_async("ws://127.0.0.1:9090").await {
        let (mut tx, _) = ws_stream.split();
        if let Err(e) = tx.send(Message::Text(payload.to_string())).await {
            error!("Notify send failed: {}", e);
        }
    } else {
        error!("Failed to connect to notifier");
    }
}
