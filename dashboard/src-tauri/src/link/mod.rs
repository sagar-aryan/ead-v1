//! Device links. Both transports carry the same protocol messages
//! (`docs/protocol.md`): Wi-Fi WebSocket is the contract link (doc 08), USB is
//! the bench fallback (DEC-005).

pub mod usb;
pub mod ws;

use tokio::sync::mpsc;

/// Which transport to use, and how to reach the device.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LinkTarget {
    /// Serial port path; `None` picks the first attached EAD device.
    Usb { port: Option<String> },
    Wifi { url: String },
}

impl LinkTarget {
    pub fn describe(&self) -> String {
        match self {
            LinkTarget::Usb { port: Some(p) } => format!("USB {p}"),
            LinkTarget::Usb { port: None } => "USB (auto-detect)".to_string(),
            LinkTarget::Wifi { url } => format!("Wi-Fi {url}"),
        }
    }
}

/// What a transport reports to the device manager.
#[derive(Debug)]
pub enum LinkEvent {
    Connected { description: String },
    Message(Vec<u8>),
    /// Byte runs that were not valid frames, cumulative (USB only).
    RejectedFrames(u64),
    /// The transport stopped. The manager decides whether to retry.
    Disconnected { reason: String },
}

pub type CommandRx = mpsc::Receiver<Vec<u8>>;
pub type EventTx = mpsc::Sender<LinkEvent>;

/// Runs one connection attempt to completion. Returns when the link closes or
/// `commands` is dropped.
pub async fn run(target: LinkTarget, commands: CommandRx, events: EventTx) {
    match target {
        LinkTarget::Usb { port } => usb::run(port, commands, events).await,
        LinkTarget::Wifi { url } => ws::run(url, commands, events).await,
    }
}
