//! Wi-Fi link: binary WebSocket messages to the device access point
//! (doc 08 §1, `ws://192.168.4.1:8080/ws`). One protocol message per frame, so
//! no COBS framing is needed here.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use super::{CommandRx, EventTx, LinkEvent};

pub const DEFAULT_URL: &str = "ws://192.168.4.1:8080/ws";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
/// STATUS alone arrives five times a second, so this long with nothing means the
/// link is gone even while the socket looks open. Waiting for the OS's TCP timeout
/// took minutes, longer than the device's 4.5 min backfill ring (F-29).
const SILENT_AFTER: Duration = Duration::from_secs(5);

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
    let silence = tokio::time::sleep(SILENT_AFTER);
    tokio::pin!(silence);
    let reason = loop {
        tokio::select! {
            command = commands.recv() => match command {
                // A send into a stalled connection waits on the socket, so it is
                // bounded too, or the silence below could never be noticed.
                Some(frame) => {
                    let send = sink.send(Message::Binary(frame.into()));
                    match tokio::time::timeout(SILENT_AFTER, send).await {
                        Ok(Ok(())) => {}
                        Ok(Err(err)) => break Some(format!("{url}: send failed: {err}")),
                        Err(_) => break Some(format!("{url}: send stalled; the device is not taking data")),
                    }
                }
                None => break None, // manager dropped the link
            },
            () = &mut silence => {
                break Some(format!("{url}: nothing from the device for {} s", SILENT_AFTER.as_secs()));
            }
            incoming = source.next() => {
                silence.as_mut().reset(tokio::time::Instant::now() + SILENT_AFTER);
                match incoming {
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
                }
            }
        }
    };

    if let Some(reason) = reason {
        let _ = events.send(LinkEvent::Disconnected { reason }).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_that_goes_quiet_is_disconnected() {
        // F-29: the socket stays open and nothing arrives, as when the laptop
        // stays associated but the radio link is gone.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("ws://{}/ws", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            let socket = tokio_tungstenite::tungstenite::accept(socket).unwrap();
            std::thread::sleep(SILENT_AFTER + Duration::from_secs(1));
            drop(socket);
        });
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let reason = runtime.block_on(async {
            let (_commands_tx, commands) = tokio::sync::mpsc::channel(4);
            let (events, mut events_rx) = tokio::sync::mpsc::channel(4);
            let started = tokio::time::Instant::now();
            tokio::spawn(run(url, commands, events));
            assert!(matches!(
                events_rx.recv().await,
                Some(LinkEvent::Connected { .. })
            ));
            let Some(LinkEvent::Disconnected { reason }) = events_rx.recv().await else {
                panic!("expected a disconnect");
            };
            assert!(started.elapsed() < SILENT_AFTER + Duration::from_secs(1));
            reason
        });
        assert!(reason.contains("nothing from the device"), "{reason}");
        server.join().unwrap();
    }
}
