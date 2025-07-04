use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use tracing::{error, info};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let addr = "127.0.0.1:9090";
    let listener = TcpListener::bind(addr).await.expect("Can't bind");
    info!("Notifier WS listening on {}", addr);

    // Store connected clients
    let clients: Arc<Mutex<HashMap<Uuid, tokio::sync::mpsc::UnboundedSender<Message>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let peer_clients = clients.clone();

        tokio::spawn(async move {
            let ws_stream = accept_async(stream).await.expect("Accept failed");
            let (mut ws_tx, mut ws_rx) = ws_stream.split();

            // Channel to push logs to this client
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
            let client_id = Uuid::new_v4();
            peer_clients.lock().unwrap().insert(client_id, tx);

            // Task to send logs to this client
            let send_task = tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if let Err(e) = ws_tx.send(msg).await {
                        error!("Send to client failed: {}", e);
                        break;
                    }
                }
            });

            // Task to handle incoming logs and broadcast to all clients
            let recv_task = {
                let clients = peer_clients.clone();
                tokio::spawn(async move {
                    while let Some(msg) = ws_rx.next().await {
                        if let Ok(Message::Text(txt)) = msg {
                            info!("[NOTIFY]--> {}", txt);

                            // Broadcast to all clients
                            let peers = clients.lock().unwrap();
                            for (_id, sender) in peers.iter() {
                                let _ = sender.send(Message::Text(txt.clone()));
                            }

                            // Integrate email/sms logic here if needed
                        }
                    }

                    // Clean up on disconnect
                    clients.lock().unwrap().remove(&client_id);
                })
            };

            // Await both tasks
            let _ = tokio::join!(send_task, recv_task);
        });
    }
}
