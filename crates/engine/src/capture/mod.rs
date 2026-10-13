//! Packet sources: every device of this machine, or a capture file replayed in
//! real time. Both push TCP payloads into the same channel the dispatcher reads.

pub(crate) mod pcap;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use a2tools::capture::captured_payload::CapturedPayload;

use crate::clock::now_ms;
use pcap::{Device, Next, PcapLib};

/// Physical adapters open this long after the virtual ones, so a relay's
/// loopback flow gets the chance to be locked first.
const PHYSICAL_DEVICE_DELAY: Duration = Duration::from_millis(1500);

pub(crate) struct Capture {
    pub threads: Vec<JoinHandle<()>>,
}

pub(crate) fn start_live(
    tx: SyncSender<CapturedPayload>,
    running: Arc<AtomicBool>,
) -> Result<Capture, String> {
    let lib = Arc::new(PcapLib::load()?);
    let devices = lib.devices()?;
    if devices.is_empty() {
        return Err("no capture device with an address".into());
    }
    for d in &devices {
        tracing::info!("capture device: {} (loopback={})", d.label(), d.is_loopback);
    }

    let (virtual_devices, physical): (Vec<Device>, Vec<Device>) =
        devices.into_iter().partition(Device::is_virtual);
    let delay = if virtual_devices.is_empty() {
        Duration::ZERO
    } else {
        PHYSICAL_DEVICE_DELAY
    };

    let mut threads: Vec<_> = virtual_devices
        .into_iter()
        .map(|d| spawn_device(lib.clone(), d, Duration::ZERO, tx.clone(), running.clone()))
        .collect();
    threads.extend(
        physical
            .into_iter()
            .map(|d| spawn_device(lib.clone(), d, delay, tx.clone(), running.clone())),
    );
    Ok(Capture { threads })
}

fn spawn_device(
    lib: Arc<PcapLib>,
    device: Device,
    delay: Duration,
    tx: SyncSender<CapturedPayload>,
    running: Arc<AtomicBool>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name(format!("capture:{}", device.label()))
        .spawn(move || {
            std::thread::sleep(delay);
            if !running.load(Ordering::Relaxed) {
                return;
            }
            let mut handle = match lib.open_live(&device.name) {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!("cannot open {}: {e}", device.label());
                    return;
                }
            };
            let link = handle.link_type();
            let label = device.label().to_string();
            while running.load(Ordering::Relaxed) {
                match handle.next() {
                    Next::Frame(frame, ts) => {
                        let ts = if (1_000_000_000_000..2_000_000_000_000).contains(&ts) {
                            ts
                        } else {
                            now_ms()
                        };
                        if let Some(payload) = pcap::tcp_payload(frame, link, &label, ts) {
                            // A full channel drops the packet rather than stall libpcap.
                            let _ = tx.try_send(payload);
                        }
                    }
                    Next::Timeout => {}
                    Next::End | Next::Error => break,
                }
            }
            tracing::info!("capture stopped on {label}");
        })
        .expect("spawn capture thread")
}

/// Replay a `.pcap`/`.pcapng` at `speed` times real time, stamping packets
/// with the wall clock so the parser's timers behave as they do live.
pub(crate) fn start_replay(
    path: &Path,
    speed: f64,
    tx: SyncSender<CapturedPayload>,
    running: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
) -> Result<Capture, String> {
    let lib = PcapLib::load()?;
    let path: PathBuf = path.to_path_buf();
    // Fail now, on the caller's thread, if the file does not open.
    drop(lib.open_file(&path)?);
    let speed = speed.clamp(0.1, 50.0);

    let thread = std::thread::Builder::new()
        .name("capture:replay".into())
        .spawn(move || {
            let mut handle = match lib.open_file(&path) {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!("replay {}: {e}", path.display());
                    finished.store(true, Ordering::Relaxed);
                    return;
                }
            };
            let link = handle.link_type();
            let started = Instant::now();
            let mut first_ts: Option<i64> = None;
            while running.load(Ordering::Relaxed) {
                match handle.next() {
                    Next::Frame(frame, ts) => {
                        let first = *first_ts.get_or_insert(ts);
                        let due =
                            Duration::from_secs_f64(((ts - first).max(0) as f64 / 1000.0) / speed);
                        if let Some(wait) = due.checked_sub(started.elapsed()) {
                            std::thread::sleep(wait);
                        }
                        if let Some(payload) = pcap::tcp_payload(frame, link, "replay", now_ms()) {
                            // Replays wait for the parser instead of dropping.
                            if tx.send(payload).is_err() {
                                break;
                            }
                        }
                    }
                    Next::Timeout => {}
                    Next::End | Next::Error => break,
                }
            }
            finished.store(true, Ordering::Relaxed);
            tracing::info!("replay of {} finished", path.display());
        })
        .map_err(|e| e.to_string())?;
    Ok(Capture {
        threads: vec![thread],
    })
}
