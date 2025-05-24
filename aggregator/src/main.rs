use chrono::{NaiveDateTime, TimeZone, Utc};
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
    let listener = TcpListener::bind(addr).await.expect("Can't listen");
    info!("Aggregator WS listening on {}", addr);

    let peers: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        info!("New TCP connection from {}", addr);

        let ws_stream = accept_async(stream).await.expect("Failed to accept");
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
                        if let Some(log) = parse_log_line(&txt) {
                            print!("[LOG] {:?} ", log);

                            info!("[LOG] {:?}", log);

                            let critical_codes = [400, 401, 404];
                            if critical_codes.contains(&log.code) {
                                notify(&txt).await;
                            }
                        } else {
                            error!("Failed to parse log: {}", txt);
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

#[derive(Debug)]
struct ParsedLog {
    code: u16,
    level: String,
    timestamp: u64,
    service: String,
    process_name: String,
    process_id: u32,
    thread_name: String,
    thread_id: String,
    message: String,
}

fn parse_log_line(line: &str) -> Option<ParsedLog> {
    let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    if parts.len() < 9 {
        return None;
    }

    let code = parts[0].parse::<u16>().ok()?;
    let level = parts[1].to_string();
    let datetime_str = parts[2];
    let service = parts[3].to_string();
    let process_name = parts[4].to_string();
    let process_id = parts[5].parse::<u32>().ok()?;
    let thread_name = parts[6].to_string();
    let thread_id = parts[7].to_string();
    let message = parts[8..].join(" | "); 

    // Parse datetime string like "2025-05-20 11:30:05,332"
    let naive_dt = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S,%f").ok()?;
    let timestamp = Utc.from_utc_datetime(&naive_dt).timestamp() as u64;

    Some(ParsedLog {
        code,
        level,
        timestamp,
        service,
        process_name,
        process_id,
        thread_name,
        thread_id,
        message,
    })
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
