//! USB link: COBS-framed protocol messages over the device's native USB
//! Serial/JTAG port (`docs/protocol.md` §2.2).
//!
//! `serialport` is blocking, so the loop runs on a blocking thread and talks to
//! the async side through channels.

use std::time::Duration;


use super::{CommandRx, EventTx, LinkEvent};
use crate::protocol::{self, UsbDecoder};

/// USB identifiers of the XIAO ESP32-S3's native USB Serial/JTAG interface.
const VID: u16 = 0x303A;
const PID: u16 = 0x1001;
const READ_TIMEOUT: Duration = Duration::from_millis(20);
const IDLE_SLEEP: Duration = Duration::from_millis(2);

#[derive(Debug, Clone, serde::Serialize)]
pub struct UsbPortInfo {
    pub port: String,
    /// The device MAC, which the firmware reports as the USB serial number.
    pub serial_number: Option<String>,
}

/// Attached EAD devices, by USB vendor and product ID.
pub fn list_ports() -> Vec<UsbPortInfo> {
    let ports = match serialport::available_ports() {
        Ok(ports) => ports,
        Err(_) => return Vec::new(),
    };
    ports
        .into_iter()
        .filter_map(|p| match &p.port_type {
            serialport::SerialPortType::UsbPort(usb) if usb.vid == VID && usb.pid == PID => {
                Some(UsbPortInfo {
                    port: p.port_name.clone(),
                    serial_number: usb.serial_number.clone(),
                })
            }
            _ => None,
        })
        .collect()
}

pub async fn run(port: Option<String>, commands: CommandRx, events: EventTx) {
    let _ = tokio::task::spawn_blocking(move || run_blocking(port, commands, events)).await;
}

fn run_blocking(port: Option<String>, mut commands: CommandRx, events: EventTx) {
    let path = match port.or_else(|| list_ports().first().map(|p| p.port.clone())) {
        Some(path) => path,
        None => {
            let _ = events.blocking_send(LinkEvent::Disconnected {
                reason: "no EAD device found on USB".to_string(),
            });
            return;
        }
    };

    // Never touch DTR/RTS: clearing them one at a time passes through
    // DTR=0/RTS=1, which resets the ESP32-S3 (PROB-005). Linux asserts both on
    // open, which is the state the device ignores.
    let mut serial = match serialport::new(&path, 115_200).timeout(READ_TIMEOUT).open() {
        Ok(serial) => serial,
        Err(err) => {
            let _ = events
                .blocking_send(LinkEvent::Disconnected { reason: format!("{path}: {err}") });
            return;
        }
    };

    if events
        .blocking_send(LinkEvent::Connected { description: format!("USB {path}") })
        .is_err()
    {
        return;
    }

    let mut decoder = UsbDecoder::new();
    let mut reported_rejected = 0u64;
    let mut buffer = [0u8; 8192];
    // Ok(()) = the manager dropped the link and wants no further events.
    let reason: std::result::Result<(), String> = 'link: loop {
        // Outbound first: a command queued behind a read timeout would delay
        // the device's reply by that timeout.
        loop {
            match commands.try_recv() {
                Ok(message) => {
                    // The device manager hands over bare protocol messages;
                    // framing belongs to this transport.
                    if let Err(err) = serial.write_all(&protocol::usb_frame(&message)) {
                        break 'link Err(format!("{path}: write failed: {err}"));
                    }
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break 'link Ok(()),
            }
        }

        match serial.read(&mut buffer) {
            Ok(0) => std::thread::sleep(IDLE_SLEEP),
            Ok(n) => {
                for message in decoder.feed(&buffer[..n]) {
                    if events.blocking_send(LinkEvent::Message(message)).is_err() {
                        break 'link Ok(());
                    }
                }
                // Frames that fail COBS or CRC are the signal that the link is
                // corrupting data (PROB-006); report the count, do not hide it.
                if decoder.rejected() != reported_rejected {
                    reported_rejected = decoder.rejected();
                    if events
                        .blocking_send(LinkEvent::RejectedFrames(reported_rejected))
                        .is_err()
                    {
                        break 'link Ok(());
                    }
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::TimedOut => {}
            Err(err) => break 'link Err(format!("{path}: {err}")),
        }
    };

    if let Err(reason) = reason {
        let _ = events.blocking_send(LinkEvent::Disconnected { reason });
    }
}
