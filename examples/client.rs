use clap::Parser;

use futures_util::{future, pin_mut, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use futures::SinkExt;
use futures_channel::mpsc;
use git_version::git_version;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use url::Url;

const GIT_VERSION: &str = git_version!();

#[derive(Parser)]
#[clap(name = "Example Gatman client", version = GIT_VERSION)]
struct Opts {
    pub url: Url,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opts: Opts = Opts::parse();

    let (ws_stream, _) = connect_async(opts.url).await?;
    println!("websocket connected");

    let (stdin_tx, stdin_rx) = mpsc::unbounded();
    let (mut ws_tx, ws_rx) = ws_stream.split();

    let (tx, mut rx) = mpsc::unbounded();

    tokio::spawn(read_stdin(stdin_tx));
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(3));
        loop {
            tokio::select! {
                Some(m) = rx.next() => {
                    ws_tx.send(m).await.expect("tx fail");
                    interval.reset();
                }
                _ = interval.tick() => {
                    ws_tx.send(Message::Text("keep-alive".to_string())).await.expect("tx fail");
                    println!("tick");
                }
                else => {
                    println!("unknown state");
                }
            }
        }
    });

    let stdin_to_ws = stdin_rx.map(Ok).forward(tx.clone());
    let ws_to_stdout = {
        ws_rx.for_each(|message| async {
            let data = message.unwrap().into_data();
            tokio::io::stdout()
                .write_all(&data)
                .await
                .expect("failed to write stdout");
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;

    Ok(())
}

async fn read_stdin(tx: mpsc::UnboundedSender<Message>) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        let s = String::from_utf8(buf).expect("failed to read stdin");
        tx.unbounded_send(Message::text(s)).unwrap();
    }
}
