use futures::{SinkExt, StreamExt};
use sqlx::postgres::PgPoolOptions;
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

    // Setup SQLx Postgres pool (tweak max connections as needed)
    let db_url = std::env::var("DATABASE_URL").expect("Set DATABASE_URL");
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("DB connect failed");

    type ClientSender = tokio::sync::mpsc::UnboundedSender<Message>;
    let clients: Arc<Mutex<HashMap<Uuid, (ClientSender, ClientType)>>> =
        Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        let clients = clients.clone();
        let db_pool = db_pool.clone(); // pool is internally Arc, clone is cheap
        info!("New TCP connection from {}", addr);

        tokio::spawn(async move {
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    error!("Failed to accept WS: {e}");
                    return;
                }
            };
            let (mut ws_tx, mut ws_rx) = ws_stream.split();

            // First message: SUBSCRIBE:all or SUBSCRIBE:important
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
                        error!("Send failed: {e}");
                        break;
                    }
                }
            });

            let recv_clients = clients.clone();
            let db_pool2 = db_pool.clone();
            let recv_task = tokio::spawn(async move {
                while let Some(msg) = ws_rx.next().await {
                    match msg {
                        Ok(Message::Text(txt)) => {
                            info!("Received log: {}", txt);

                            // Save log into Postgres asynchronously (fire & forget)
                            let txt_clone = txt.clone();
                            let pool = db_pool2.clone();
                            tokio::spawn(async move {
                                // Note: create table logs (id serial primary key, message text not null, received_at timestamptz default now());
                                if let Err(e) = sqlx::query!(
                                    "INSERT INTO logs (message) VALUES ($1)",
                                    txt_clone
                                )
                                .execute(&pool)
                                .await
                                {
                                    error!("DB insert failed: {e}");
                                }
                            });

                            let maybe_code = try_extract_code(&txt);
                            let is_critical = maybe_code
                                .map_or(false, |code| [400, 401, 402, 403, 404].contains(&code));

                            // Clone all peers so we don't hold the lock across await
                            let peers: Vec<(ClientSender, ClientType)> = {
                                let lock = recv_clients.lock().unwrap();
                                lock.values().map(|(s, t)| (s.clone(), *t)).collect()
                            };

                            for (sender, typ) in peers {
                                if typ == ClientType::All
                                    || (is_critical && typ == ClientType::ImportantOnly)
                                {
                                    let _ = sender.send(Message::Text(txt.clone()));
                                }
                            }
                            // TODO: hook for alert
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
