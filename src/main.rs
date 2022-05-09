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

    let driver = Drive::new(
        opts.at,
        opts.en_pin,
        opts.dir_pin,
        opts.clock_pin,
        opts.data_pin,
    )?;
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
    // todo;; there is a pipe function available
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
    // to_client
    //     .send(Message::text("hello"))
    //     .expect("failed to init");

    // receive messages from the ws client and hand them off to the gateman
    while let Ok(result) = tokio::time::timeout(Duration::from_secs(50), from_client.next()).await {
        match result {
            Some(Ok(msg)) if msg.is_text() => {
                let t = msg.to_str().unwrap().trim();
                match t {
                    "ping" => {
                        gm.sender.send(gate::Command::Nop).await.unwrap();
                    }
                    "close" => {
                        println!("cmd: closing");
                        to_client.send(Message::text("closing:0")).unwrap();
                    }
                    v => {
                        let to: u8 = v.parse().unwrap();
                        println!("cmd: open to {}", to);
                        gm.sender.send(gate::Command::Open(to)).await.unwrap();
                        to_client
                            .send(Message::text(format!("moving:{}", to)))
                            .unwrap();
                    }
                }
            }
            Some(Ok(msg)) if msg.is_close() => {
                gm.sender.send(gate::Command::Close).await.unwrap();
                break;
            }
            err => {
                println!("--- unsupported message {:?} ---", err);
                gm.sender.send(gate::Command::Close).await.unwrap();
                // gate.send("[e]close".to_string()).unwrap();
                break;
            }
        };
    }
    drop(h);
    eprintln!("shutting down")
}
