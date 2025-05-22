use futures::{SinkExt, StreamExt}; 
use serde::Serialize;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::info;

#[derive(Serialize)]
struct LogEntry<'a> {
    code: u16, 
    service: &'a str,
    timestamp: u64,
    message: &'a str,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    if let Ok((ws_stream, _)) = connect_async("ws://127.0.0.1:8080").await {
        let (mut tx, _) = ws_stream.split();
        info!("Service A connected");
        loop {
            let codes = [100, 200, 404, 500, 401, 400];
            let code = codes[rand::random::<usize>() % codes.len()];
            let log = LogEntry {
                code,
                service: "ServiceA",
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                message: "Sample log message",
            };
            let payload = serde_json::to_string(&log).unwrap();
            tx.send(Message::Text(payload)).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}
