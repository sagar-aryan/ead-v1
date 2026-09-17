//! Live view feed. The device streams 100 Hz frames; the UI renders at 20 Hz
//! (doc 11 §3), so frames are aggregated per tick here and the full-rate data
//! stays in the store.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::ipc::Channel;

use crate::protocol::{config::DeviceConfigSection, RawFrame};

pub const TICK: Duration = Duration::from_millis(50);

/// One 20 Hz update: means over the tick, in anatomical units (doc 04 §1).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LiveTick {
    pub device_time_us: u64,
    pub frame_index: u32,
    pub frames: u32,
    pub foot_accel_g: [f32; 3],
    pub foot_gyro_dps: [f32; 3],
    pub shank_accel_g: [f32; 3],
    pub shank_gyro_dps: [f32; 3],
    /// |a| per sensor: 1.0 at rest, a quick check that scaling and mounting are right.
    pub foot_accel_magnitude_g: f32,
    pub shank_accel_magnitude_g: f32,
    /// Status flags seen anywhere in this tick, OR-ed (`docs/protocol.md` §5.4).
    pub status_flags: u16,
    /// False while the device configuration is unknown, so units are unscaled.
    pub anatomical: bool,
}

#[derive(Default)]
struct Pending {
    frames: Vec<RawFrame>,
    config: Option<Arc<DeviceConfigSection>>,
}

pub struct LiveHub {
    pending: Mutex<Pending>,
    subscriber: Mutex<Option<Channel<LiveTick>>>,
}

impl LiveHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { pending: Mutex::new(Pending::default()), subscriber: Mutex::new(None) })
    }

    pub fn subscribe(&self, channel: Channel<LiveTick>) {
        *self.subscriber.lock().expect("live subscriber") = Some(channel);
    }

    pub fn unsubscribe(&self) {
        *self.subscriber.lock().expect("live subscriber") = None;
    }

    pub fn set_config(&self, config: Option<Arc<DeviceConfigSection>>) {
        self.pending.lock().expect("live pending").config = config;
    }

    pub fn push(&self, frames: &[RawFrame]) {
        let mut pending = self.pending.lock().expect("live pending");
        // Only a subscriber consumes these; without one, drop them rather than grow.
        if self.subscriber.lock().expect("live subscriber").is_none() {
            pending.frames.clear();
            return;
        }
        pending.frames.extend_from_slice(frames);
    }

    /// Builds one tick from the frames collected since the last call.
    fn take_tick(&self) -> Option<LiveTick> {
        let mut pending = self.pending.lock().expect("live pending");
        if pending.frames.is_empty() {
            return None;
        }
        let frames = std::mem::take(&mut pending.frames);
        let config = pending.config.clone();
        drop(pending);

        let last = frames.last().copied().expect("non-empty");
        let count = frames.len() as f32;
        let mut sums = [[0f32; 3]; 4];
        let mut status_flags = 0u16;
        for frame in &frames {
            status_flags |= frame.status;
            let (foot_accel, foot_gyro, shank_accel, shank_gyro) = match config.as_deref() {
                Some(config) => {
                    let (fa, fg) = config.foot_anatomical(&frame.foot);
                    let (sa, sg) = config.shank_anatomical(&frame.shank);
                    (fa, fg, sa, sg)
                }
                // No configuration yet: show raw counts rather than invent a scale.
                None => (
                    [frame.foot[0] as f32, frame.foot[1] as f32, frame.foot[2] as f32],
                    [frame.foot[3] as f32, frame.foot[4] as f32, frame.foot[5] as f32],
                    [frame.shank[0] as f32, frame.shank[1] as f32, frame.shank[2] as f32],
                    [frame.shank[3] as f32, frame.shank[4] as f32, frame.shank[5] as f32],
                ),
            };
            for (slot, value) in sums.iter_mut().zip([foot_accel, foot_gyro, shank_accel, shank_gyro])
            {
                for axis in 0..3 {
                    slot[axis] += value[axis];
                }
            }
        }
        let mean = |sum: [f32; 3]| [sum[0] / count, sum[1] / count, sum[2] / count];
        let magnitude = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let foot_accel_g = mean(sums[0]);
        let shank_accel_g = mean(sums[2]);
        Some(LiveTick {
            device_time_us: last.timestamp_us,
            frame_index: last.frame_index,
            frames: frames.len() as u32,
            foot_accel_g,
            foot_gyro_dps: mean(sums[1]),
            shank_accel_g,
            shank_gyro_dps: mean(sums[3]),
            foot_accel_magnitude_g: magnitude(foot_accel_g),
            shank_accel_magnitude_g: magnitude(shank_accel_g),
            status_flags,
            anatomical: config.is_some(),
        })
    }

    fn emit(&self, tick: LiveTick) {
        let subscriber = self.subscriber.lock().expect("live subscriber");
        if let Some(channel) = subscriber.as_ref() {
            // A closed channel means the webview reloaded; the next subscribe replaces it.
            let _ = channel.send(tick);
        }
    }
}

/// Runs the 20 Hz emitter until the app shuts down.
pub async fn run(hub: Arc<LiveHub>, mut stop: tokio::sync::watch::Receiver<bool>) {
    let mut ticker = tokio::time::interval(TICK);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if let Some(tick) = hub.take_tick() {
                    hub.emit(tick);
                }
            }
            _ = stop.changed() => {
                if *stop.borrow() {
                    return;
                }
            }
        }
    }
}
