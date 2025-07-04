use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq)]
enum ClientType {
    All,
    ImportantOnly,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Can't listen");
    info!("Log server running on {}", addr);

    type ClientSender = tokio::sync::mpsc::UnboundedSender<Message>;
    let clients: Arc<Mutex<HashMap<Uuid, (ClientSender, ClientType)>>> =
        Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        let clients = clients.clone();
        info!("New TCP connection from {}", addr);

        tokio::spawn(async move {
            let ws_stream = accept_async(stream).await.expect("Failed to accept");
            let (mut ws_tx, mut ws_rx) = ws_stream.split();

            // First message from client should be: "SUBSCRIBE:all" or "SUBSCRIBE:important"
            let subscription_type = match ws_rx.next().await {
                Some(Ok(Message::Text(sub))) if sub == "SUBSCRIBE:important" => {
                    ClientType::ImportantOnly
                }
                _ => ClientType::All, // default fallback
            };

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
            let client_id = Uuid::new_v4();
            clients
                .lock()
                .unwrap()
                .insert(client_id, (tx.clone(), subscription_type));

            let send_task = tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if let Err(e) = ws_tx.send(msg).await {
                        error!("Send failed: {}", e);
                        break;
                    }
                }
            });

            let recv_clients = clients.clone();
            let recv_task = tokio::spawn(async move {
                while let Some(msg) = ws_rx.next().await {
                    match msg {
                        Ok(Message::Text(txt)) => {
                            info!("Received log: {}", txt);
                            info!("Received log: {}", txt);

                            let txt_clone = txt.clone();
                            tokio::spawn(async move {
                                let client = reqwest::Client::new();
                                let _ = client
                                    .post("http://localhost:3000/log")
                                    .json(&serde_json::json!({ "message": txt_clone }))
                                    .send()
                                    .await;
                            });
                            let maybe_code = try_extract_code(&txt);
                            let is_critical = maybe_code
                                .map_or(false, |code| [400, 401, 402, 403, 404].contains(&code));

                            let peers = recv_clients.lock().unwrap();
                            for (_, (sender, typ)) in peers.iter() {
                                if *typ == ClientType::All
                                    || (is_critical && *typ == ClientType::ImportantOnly)
                                {
                                    let _ = sender.send(Message::Text(txt.clone()));
                                }
                            }

                            // TODO: hook for email/sms alert
                        }
                        Ok(Message::Close(_)) | Err(_) => {
                            break;
                        }
                        _ => {}
                    }
                }

                recv_clients.lock().unwrap().remove(&client_id);
            });

            let _ = tokio::join!(send_task, recv_task);
        });
    }
}

fn try_extract_code(text: &str) -> Option<u16> {
    for word in text.split(|c: char| !c.is_numeric()) {
        if let Ok(code) = word.parse::<u16>() {
            return Some(code);
        }
    }
    None
}
