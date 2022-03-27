use futures_util::{SinkExt, TryFutureExt};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_stream::wrappers::UnboundedReceiverStream;

use gateman::api;
use gateman::gate::GatemanRef;
use warp::filters::ws::{Message, WebSocket};
use warp::Filter;

/// # Gateman Webservice
/// Websocket based service that provides remote gate control.
///
/// ## Goal: Bi-directional connection that supports
/// 1. Ensuring that the client device is connected
/// 2. Receiving commands from the client device
/// 3. Sending status updates to the client device
///
/// ## Fail-safe modes must be in place to support
/// 1. Shutting the sytem down in a controlled manner if client is non-responsive or disconnects
///
#[tokio::main]
async fn main() {
    let gm = GatemanRef::new();
    let gate = warp::any().map(move || gm.clone());

    let routes = warp::path("gate")
        .and(warp::ws())
        .and(gate)
        .map(|ws: warp::ws::Ws, tx| ws.on_upgrade(|websocket| router(websocket, tx)));

    eprintln!("websocket ready");
    warp::serve(routes).run(([127, 0, 0, 1], 9000)).await;
}

// handles the routing of messages to and from the websocket connection
async fn router(websocket: WebSocket, gm: GatemanRef) {
    use futures_util::StreamExt;

    let (mut ws_tx, mut from_client) = websocket.split();
    let (to_client, rx) = mpsc::unbounded_channel();

    eprintln!("connected");

    let mut rx = UnboundedReceiverStream::new(rx);
    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            ws_tx
                .send(message)
                .unwrap_or_else(|e| {
                    eprintln!("websocket send error: {}", e);
                })
                .await;
        }
    });

    // fire off an initial message to the client
    to_client
        .send(Message::text("hello"))
        .expect("failed to init");

    // receive messages from the ws client and hand them off to the gateman
    while let Some(result) = from_client.next().await {
        match result {
            Ok(msg) if msg.is_text() => {
                gm.sender.send(api::Command::Open(1)).await.unwrap();
                //gate.send(msg.to_str().unwrap().to_string()).unwrap();
            }
            Ok(msg) if msg.is_close() => {
                gm.sender.send(api::Command::Close).await.unwrap();
                // gate.send("close".to_string()).unwrap();
            }
            Err(_) => {
                gm.sender.send(api::Command::Close).await.unwrap();
                // gate.send("[e]close".to_string()).unwrap();
                break;
            }
            _ => eprintln!("unsupported message type"),
        };
    }
    eprintln!("shutting down")
}

// represents the gate actor which receives messages and shuts the gate after an inactivity timeout
fn mock_gateman(mbox: UnboundedReceiver<String>) {
    use tokio_stream::StreamExt;
    let mut rx = UnboundedReceiverStream::new(mbox);

    tokio::task::spawn(async move {
        while let Ok(message) = tokio::time::timeout(Duration::from_secs(5), rx.next()).await {
            match message {
                Some(message) => {
                    eprintln!("gate rx-comm: {}", message.trim());
                }
                None => {
                    eprintln!("gate rx-term: closing");
                    return;
                }
            }
        }
        eprintln!("gate timeout: closing");
    });
}
