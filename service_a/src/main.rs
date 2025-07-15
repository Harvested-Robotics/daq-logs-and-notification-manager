use futures::{SinkExt, StreamExt};
use rand::seq::SliceRandom;
use std::process;
use std::thread;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    if let Ok((ws_stream, _)) = connect_async("ws://127.0.0.1:8080").await {
        let (mut tx, _) = ws_stream.split();
        info!("Service A connected");

        // Simulated data
        let level_names = ["INFO", "DEBUG", "ERROR", "CRITICAL"];
        let process_name = "MainProcess";
        let thread_name = "MainThread";
        let thread_id = "threadId";

        loop {
            let codes: [i32; 16] = [100, 200, 404, 500, 401,402, 403, 400, 101, 102, 103, 104, 105, 106, 107, 108];
            let code: i32 = *codes.choose(&mut rand::thread_rng()).unwrap();
            let level: &&str = level_names.choose(&mut rand::thread_rng()).unwrap();
            let timestamp: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
            let datetime: String = timestamp.format("%Y-%m-%d %H:%M:%S,%3f").to_string();
            let service: &'static str = "hlp.camera.CameraConsumer";
            let pid = process::id();

            let message = format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {}",
                code,
                level,
                datetime,
                service,
                process_name,
                pid,
                thread_name,
                thread_id,
                format!("Sample message for code {}", code)
            );

            tx.send(Message::Text(message)).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }
}
