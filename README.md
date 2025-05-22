# daq-logs-and-notification-manager
# 🚀 Distributed Log Aggregation System in Rust

A robust, concurrent log aggregation system built in Rust using asynchronous WebSockets. The system is composed of three log-generating services, an aggregator to collect and process logs, and a notifier that alerts on critical log entries using a **priority queue**.

---

## 🧱 Components Overview

This system consists of:

- **Service A / B / C**: Simulate different microservices emitting structured JSON logs periodically.
- **Aggregator**: Collects logs over WebSockets, parses and validates them, then forwards critical ones.
- **Notifier**: Receives critical logs and enqueues them into a **priority queue**, then sends alerts in order of severity.

---

## 📈 System Architecture
```text
+-------------+      WebSocket       +-------------+
|             |  ----------------->  |             |
|  Service A  |                      |             |
|             |                      |             |
+-------------+                      |             |
                                     |             |
+-------------+      WebSocket       |             |
|             |  ----------------->  |             |
|  Service B  |                      | Aggregator  |
|             |                      |             |
+-------------+                      |             |
                                     |             |
+-------------+      WebSocket       |             |
|             |  ----------------->  |             |
|  Service C  |                      |             |
|             |                      +------+------+
+-------------+                             |
                                            |
                                            | Parse & Classify Logs
                                            v
                                  +---------+----------+
                                  |                    |
                                  |     Notifier       |
                                  |                    |
                                  +---------+----------+
                                            |
           +------------+-------------------+---------------------+
           |            |                                         |
           v            v                                         v
+----------------+ +----------------+                   +----------------+
|  Critical Q    | |   High Q       |                   |     Low Q      |
| (5xx & 404)    | | (400, 401, etc)|                   |  (info/debug)  |
+-------+--------+ +--------+-------+                   +--------+-------+
        |                   |                                   |
        +---------+---------+                                   |
                  |                                             |
                  v                                             v
         +--------+--------+                          +---------+----------+
         | Alert Engine /  |                          |   Log Storage in   |
         | Notifications   |                          |     PostgreSQL     |
         +--------+--------+                          +---------+----------+
                  |                                             |
                  |                                             v
                  +---------------------------------->+-------------------+
                                                     |  Cloud Sync Task   |
                                                     | (Periodic Uploads) |
                                                     +-------------------+



---
```

## 🧱 Components

### 🛰️ Services (A, B, C)
- Each service streams logs to the **Aggregator** via WebSocket.
- Services are written in Rust using `tokio`, `warp`, or `axum` for async WebSocket handling.

### 📦 Aggregator
- Receives and parses incoming logs.
- Classifies logs by severity.
- Sends logs to the `Notifier`.

### 🧠 Notifier
- Buffers logs using **three priority queues**:
  - **Critical Queue**: HTTP 5xx and 404
  - **High Queue**: HTTP 400, 401, 403
  - **Low Queue**: info/debug logs
- Alerts critical/high severity logs to the notification system.

### 📣 Alert Engine
- Sends out alerts (Slack, email, SMS, etc.).
- Configurable and pluggable alert destinations.
- Can use libraries like `lettre`, `reqwest`, or `slack-morphism`.

### 🗃️ PostgreSQL Storage
- Logs are stored with metadata (timestamp, source service, severity).
- Ensures persistence and future querying.

### ☁️ Cloud Sync Task
- Periodically uploads logs from PostgreSQL to cloud storage (e.g., AWS S3, GCP).
- Written as a scheduled Rust task using `cron`, `tokio`, or `async-std`.

---

## 🔧 Technologies

| Layer           | Tools / Frameworks            | Status     |
|----------------|-------------------------------|------------|
| Language        | Rust                          | ✅ Complete |
| Async Runtime   | Tokio                         | ✅ Complete |
| WebSocket       | `tokio-tungstenite`, `axum`   | ✅ Complete |
| Queues          | `crossbeam`, `tokio::sync`    | 📝 TODO |
| DB              | PostgreSQL (`sqlx`, `diesel`) | 📝 TODO |
| Notification    | `lettre`, `reqwest`           | 📝 TODO     |
| Scheduler       | `tokio_cron_scheduler`        | 📝 TODO     |
| Cloud Storage   | AWS SDK, GCP SDK (Rust)       | 📝 TODO     |


---

## 🚀 How It Works

1. Services stream logs to the aggregator.
2. Aggregator classifies and pushes logs to the notifier.
3. Notifier queues logs based on severity.
4. Critical/High logs trigger alerts via the alert engine.
5. All logs are stored in PostgreSQL.
6. A sync task uploads logs periodically to cloud storage.

---

## 🛡️ Features

- ✅ Real-time WebSocket-based log ingestion
- ✅ Priority-based log queuing
- ✅ Robust alerting system
- ✅ PostgreSQL for durable storage
- ✅ Periodic cloud sync support
- ✅ Easily extensible for new services or log types
