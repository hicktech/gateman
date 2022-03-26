use futures_util::{SinkExt, StreamExt, TryFutureExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::Filter;
use warp::filters::ws::{Message, WebSocket};

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
    let routes = warp::path("gate")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(|websocket| connection(websocket))
        });

    eprintln!("websocket ready");
    warp::serve(routes).run(([127, 0, 0, 1], 9000)).await;
}

async fn connection(websocket: WebSocket) {
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

    to_client.send(Message::text("hello")).expect("failed to init");

    while let Some(result) = from_client.next().await {
        match result {
            Ok(msg) if msg.is_text() => {
                // control gate
                eprintln!("txt: {}", msg.to_str().unwrap());
            }
            Ok(msg) if msg.is_close() => {
                // close gate
                eprintln!("closed!!!!");
            }
            Err(e) => {
                // close gate
                eprintln!("websocket error(uid={}):", e);
                break;
            }
            _ => eprintln!("unsupported message type"),
        };
    }

    eprintln!("shutting down")
}
