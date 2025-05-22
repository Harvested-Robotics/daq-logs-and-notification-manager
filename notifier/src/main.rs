use futures::StreamExt;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let addr = "127.0.0.1:9090";
    let listener = TcpListener::bind(addr).await.expect("Can't bind");
    info!("Notifier WS listening on {}", addr);
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let mut ws = accept_async(stream).await.expect("Accept failed");
            while let Some(msg) = ws.next().await {
                if let Ok(Message::Text(txt)) = msg {
                    info!("[NOTIFY]--> {}", txt);
                    // integrate email/sms here
                }
            }
        });
    }
}
