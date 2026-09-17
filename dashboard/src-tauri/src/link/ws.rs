//! Wi-Fi link: binary WebSocket messages to the device access point
//! (doc 08 §1, `ws://192.168.4.1:8080/ws`). One protocol message per frame, so
//! no COBS framing is needed here.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use super::{CommandRx, EventTx, LinkEvent};

pub const DEFAULT_URL: &str = "ws://192.168.4.1:8080/ws";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

pub async fn run(url: String, mut commands: CommandRx, events: EventTx) {
    let connect = tokio_tungstenite::connect_async(&url);
    let stream = match tokio::time::timeout(CONNECT_TIMEOUT, connect).await {
        Ok(Ok((stream, _response))) => stream,
        Ok(Err(err)) => {
            let _ = events
                .send(LinkEvent::Disconnected { reason: format!("{url}: {err}") })
                .await;
            return;
        }
        Err(_) => {
            let _ = events
                .send(LinkEvent::Disconnected {
                    reason: format!("{url}: no response — is the laptop joined to the device Wi-Fi?"),
                })
                .await;
            return;
        }
    };

    if events
        .send(LinkEvent::Connected { description: format!("Wi-Fi {url}") })
        .await
        .is_err()
    {
        return;
    }

    let (mut sink, mut source) = stream.split();
    let reason = loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(frame) => {
                    if let Err(err) = sink.send(Message::Binary(frame.into())).await {
                        break Some(format!("{url}: send failed: {err}"));
                    }
                }
                None => break None, // manager dropped the link
            },
            incoming = source.next() => match incoming {
                Some(Ok(Message::Binary(payload))) => {
                    if events.send(LinkEvent::Message(payload.to_vec())).await.is_err() {
                        break None;
                    }
                }
                // Ping/pong is handled by tungstenite; text and other frames are
                // not part of the protocol and are ignored.
                Some(Ok(Message::Close(_))) | None => {
                    break Some(format!("{url}: closed by device"));
                }
                Some(Ok(_)) => {}
                Some(Err(err)) => break Some(format!("{url}: {err}")),
            },
        }
    };

    if let Some(reason) = reason {
        let _ = events.send(LinkEvent::Disconnected { reason }).await;
    }
}
