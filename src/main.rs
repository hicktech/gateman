use std::time::Duration;

use clap::Parser;
use futures_util::{SinkExt, TryFutureExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::filters::ws::{Message, WebSocket};
use warp::Filter;

use gateman::cli::Opts;
use gateman::drive::Drive;
use gateman::gate;
use gateman::gate::GatemanRef;
use gateman::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts: Opts = Opts::parse();

    let driver = Drive::new(opts.at, opts.dir_pin, opts.clock_pin, opts.data_pin)?;
    let gm = GatemanRef::new(driver);
    let gate = warp::any().map(move || gm.clone());

    let routes = warp::path("gate")
        .and(warp::ws())
        .and(gate)
        .map(|ws: warp::ws::Ws, tx| ws.on_upgrade(|websocket| router(websocket, tx)));

    eprintln!(
        "websocket starting on {:?} port {}",
        opts.address, opts.port
    );
    let address: [u8; 4] = opts.address.into();
    warp::serve(routes).run((address, opts.port)).await;

    Ok(())
}

// handles the routing of messages to and from the websocket connection
async fn router(websocket: WebSocket, gm: GatemanRef) {
    use futures_util::StreamExt;

    let (mut ws_tx, mut from_client) = websocket.split();
    let (to_client, rx) = mpsc::unbounded_channel();

    eprintln!("connected");

    let mut rx = UnboundedReceiverStream::new(rx);
    let h = tokio::task::spawn(async move {
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
    while let Ok(result) = tokio::time::timeout(Duration::from_secs(5), from_client.next()).await {
        match result {
            Some(Ok(msg)) if msg.is_text() => {
                let n = msg.to_str().unwrap().trim().parse().unwrap();
                gm.sender.send(gate::Command::Open(n)).await.unwrap();
            }
            Some(Ok(msg)) if msg.is_close() => {
                gm.sender.send(gate::Command::Close).await.unwrap();
            }
            _ => {
                println!("--- unsupported message type or error ---");
                gm.sender.send(gate::Command::Close).await.unwrap();
                // gate.send("[e]close".to_string()).unwrap();
                break;
            }
        };
    }
    drop(h);
    eprintln!("shutting down")
}
//
// // represents the gate actor which receives messages and shuts the gate after an inactivity timeout
// fn _mock_gateman(mbox: UnboundedReceiver<String>) {
//     use tokio_stream::StreamExt;
//     let mut rx = UnboundedReceiverStream::new(mbox);
//
//     tokio::task::spawn(async move {
//         while let Ok(message) = tokio::time::timeout(Duration::from_secs(5), rx.next()).await {
//             match message {
//                 Some(message) => {
//                     eprintln!("gate rx-comm: {}", message.trim());
//                 }
//                 None => {
//                     eprintln!("gate rx-term: closing");
//                     return;
//                 }
//             }
//         }
//         eprintln!("gate timeout: closing");
//     });
// }
