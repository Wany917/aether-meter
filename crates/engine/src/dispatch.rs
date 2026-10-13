//! Finds the game's connection among everything captured and feeds its
//! server-to-client stream to the parser.
//!
//! Adapted from A2Tools DPS Meter (GPL-3.0), `combat/capture_dispatcher.rs` and
//! `capture/combat_port_detector.rs`. Dropped from the original: the AION2
//! window check (the signature-rate lock below is what actually decides), the
//! packet log and the upload ring.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU16, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use a2tools::capture::captured_payload::CapturedPayload;
use a2tools::capture::stream_assembler::StreamAssembler;
use a2tools::capture::stream_processor::StreamProcessor;
use a2tools::combat::data_storage::DataStorage;
use a2tools::combat::ping_tracker::PingTracker;

use crate::clock::now_ms;
use crate::gamedata::GameData;

/// The game's per-record terminator. Its first byte changed with the June 2026
/// update; both are accepted.
const SIGNATURES: [&[u8]; 2] = [&[0x0E, 0x00, 0x36], &[0x06, 0x00, 0x36]];
/// The live stream carries the terminator ~19 times a second even when idle;
/// a flow must show it this often within the window to be locked.
const SIGNATURE_LOCK_THRESHOLD: u32 = 12;
const SIGNATURE_WINDOW_MS: i64 = 3_000;
/// Before locking a non-loopback flow, give a relay's loopback copy this long
/// to show up.
const LOOPBACK_GRACE_MS: i64 = 2_500;
const STALE_CONNECTION_MS: i64 = 120_000;

/// What the rest of the engine may read about the connection.
#[derive(Default)]
pub(crate) struct LinkState {
    /// 0 while unlocked.
    locked_port: AtomicU16,
    last_parsed_ms: AtomicI64,
}

impl LinkState {
    pub fn locked_port(&self) -> Option<u16> {
        match self.locked_port.load(Ordering::Relaxed) {
            0 => None,
            port => Some(port),
        }
    }
}

struct Flow {
    assembler: StreamAssembler,
    processor: StreamProcessor,
}

pub(crate) struct Dispatcher {
    storage: Arc<DataStorage>,
    data: Arc<GameData>,
    ping: Arc<PingTracker>,
    link: Arc<LinkState>,
    lock: PortLock,
    flows: HashMap<(u16, u16), Flow>,
    signature_hits: HashMap<(u16, u16), (u32, i64)>,
    connections: ConnectionFilter,
}

impl Dispatcher {
    pub fn new(
        storage: Arc<DataStorage>,
        data: Arc<GameData>,
        ping: Arc<PingTracker>,
        link: Arc<LinkState>,
    ) -> Self {
        Self {
            storage,
            data,
            ping,
            link,
            lock: PortLock::default(),
            flows: HashMap::new(),
            signature_hits: HashMap::new(),
            connections: ConnectionFilter::default(),
        }
    }

    pub fn run(mut self, rx: Receiver<CapturedPayload>, running: Arc<AtomicBool>) {
        while running.load(Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(cap) => self.handle(cap),
                Err(RecvTimeoutError::Timeout) => self.drop_stale_lock(),
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn handle(&mut self, cap: CapturedPayload) {
        self.drop_stale_lock();

        let port = self.lock.port;
        if let Some(device) = &self.lock.device
            && !device_matches(device, cap.device_name.as_deref())
        {
            return;
        }
        if let Some(port) = port {
            if cap.src_port != port && cap.dst_port != port {
                return;
            }
            let ping_before = self.ping.current_ping_ms();
            self.ping.on_packet(&cap, port);
            if self.ping.current_ping_ms() != ping_before {
                self.mark_parsed();
            }
            // Only the server-to-client direction carries what we parse.
            if cap.src_port != port {
                return;
            }
        } else if looks_like_tls(&cap.data) || !contains_signature(&cap.data) {
            return;
        }
        if !self.connections.admit(&cap) {
            return;
        }

        let key = (
            cap.src_port.min(cap.dst_port),
            cap.src_port.max(cap.dst_port),
        );
        let unlocked = port.is_none();
        if unlocked {
            self.lock
                .register_candidate(cap.src_port, cap.device_name.as_deref());
            let now = now_ms();
            let hits = self.signature_hits.entry(key).or_insert((0, now));
            if now - hits.1 > SIGNATURE_WINDOW_MS {
                hits.0 = 0;
            }
            hits.0 += 1;
            hits.1 = now;
        }

        let flow = self.flows.entry(key).or_insert_with(|| {
            let mut processor = StreamProcessor::new(
                self.storage.clone(),
                self.data.skills.clone(),
                self.data.npcs.clone(),
            );
            processor.set_dot_skill_ids(self.data.dot_skill_ids.clone());
            Flow {
                assembler: StreamAssembler::new(),
                processor,
            }
        });
        let parsed = flow.assembler.process_chunk(&cap.data, &mut flow.processor);

        let rate_reached = self
            .signature_hits
            .get(&key)
            .is_some_and(|h| h.0 >= SIGNATURE_LOCK_THRESHOLD);
        if unlocked && rate_reached {
            self.lock
                .confirm(cap.src_port, cap.dst_port, cap.device_name.as_deref());
            if let Some(locked) = self.lock.port {
                tracing::info!("locked on the game's connection (server port {locked})");
                self.link.locked_port.store(locked, Ordering::Relaxed);
                // A relay duplicates the stream on other flows: keep only this one.
                self.flows.retain(|k, _| *k == key);
                self.signature_hits.clear();
            }
        }
        if parsed {
            self.mark_parsed();
        }
    }

    fn mark_parsed(&self) {
        self.link.last_parsed_ms.store(now_ms(), Ordering::Relaxed);
    }

    fn drop_stale_lock(&mut self) {
        if self.lock.port.is_none() {
            return;
        }
        let last = self.link.last_parsed_ms.load(Ordering::Relaxed);
        if last > 0 && now_ms() - last > STALE_CONNECTION_MS {
            tracing::info!(
                "no game traffic for {} s, unlocking",
                STALE_CONNECTION_MS / 1000
            );
            self.lock = PortLock::default();
            self.link.locked_port.store(0, Ordering::Relaxed);
            self.link.last_parsed_ms.store(0, Ordering::Relaxed);
            self.ping.reset();
            self.flows.clear();
            self.signature_hits.clear();
        }
    }
}

#[derive(Default)]
struct PortLock {
    port: Option<u16>,
    device: Option<String>,
    candidates: HashMap<u16, Option<String>>,
    first_candidate_ms: i64,
}

impl PortLock {
    fn register_candidate(&mut self, port: u16, device: Option<&str>) {
        let device = trimmed(device);
        if self.port.is_some() {
            // The same stream seen again on a loopback adapter: prefer it.
            if self.port == Some(port)
                && device.as_deref().is_some_and(is_loopback)
                && !self.device.as_deref().is_some_and(is_loopback)
            {
                self.device = device;
            }
            return;
        }
        if self.first_candidate_ms == 0 {
            self.first_candidate_ms = now_ms();
        }
        self.candidates.entry(port).or_insert(device);
    }

    fn confirm(&mut self, port_a: u16, port_b: u16, device: Option<&str>) {
        if self.port.is_some() {
            return;
        }
        let Some(port) = [port_a, port_b]
            .into_iter()
            .find(|p| self.candidates.contains_key(p))
        else {
            return;
        };
        let device = trimmed(device).or_else(|| self.candidates.get(&port).cloned().flatten());
        let on_loopback = device.as_deref().is_some_and(is_loopback);
        if !on_loopback && now_ms() - self.first_candidate_ms < LOOPBACK_GRACE_MS {
            return;
        }
        self.port = Some(port);
        self.device = device;
        self.candidates.clear();
    }
}

/// The game server also speaks TLS from its port on a second connection. Such
/// a connection is left out whole, decided on its first segment.
#[derive(Default)]
struct ConnectionFilter {
    seen: HashSet<(u16, u16)>,
    tls: HashSet<(u16, u16)>,
}

impl ConnectionFilter {
    fn admit(&mut self, cap: &CapturedPayload) -> bool {
        let connection = (
            cap.src_port.min(cap.dst_port),
            cap.src_port.max(cap.dst_port),
        );
        if self.seen.insert(connection) && is_tls_record(&cap.data) {
            self.tls.insert(connection);
        }
        !self.tls.contains(&connection)
    }
}

fn looks_like_tls(data: &[u8]) -> bool {
    matches!(data, [0x14..=0x17, 0x03, 0x00..=0x04, ..])
}

fn is_tls_record(data: &[u8]) -> bool {
    data.len() >= 5
        && looks_like_tls(data)
        && (1..=16_384 + 256).contains(&usize::from(u16::from_be_bytes([data[3], data[4]])))
}

pub(crate) fn contains_signature(data: &[u8]) -> bool {
    SIGNATURES
        .iter()
        .any(|s| data.windows(s.len()).any(|w| w == *s))
}

fn is_loopback(device: &str) -> bool {
    device.to_lowercase().contains("loopback")
}

fn trimmed(device: Option<&str>) -> Option<String> {
    device
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(str::to_string)
}

fn device_matches(locked: &str, device: Option<&str>) -> bool {
    device.is_some_and(|d| d.trim().eq_ignore_ascii_case(locked))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_records_are_told_from_game_frames() {
        assert!(is_tls_record(&[0x17, 0x03, 0x03, 0x05, 0x7a, 0x88]));
        assert!(is_tls_record(&[0x16, 0x03, 0x03, 0x00, 0x5d]));
        assert!(!is_tls_record(&[0x34, 0x21, 0x36, 0x01, 0x00, 0x00]));
        assert!(!is_tls_record(&[0x17, 0x03, 0x03, 0xff, 0xff]));
    }

    #[test]
    fn a_loopback_candidate_locks_at_once_and_others_wait() {
        let mut lock = PortLock::default();
        lock.register_candidate(13328, Some("Ethernet"));
        lock.confirm(13328, 50000, Some("Ethernet"));
        assert_eq!(lock.port, None, "waits for a loopback copy");

        let mut lock = PortLock::default();
        lock.register_candidate(13328, Some("Npcap Loopback Adapter"));
        lock.confirm(13328, 50000, Some("Npcap Loopback Adapter"));
        assert_eq!(lock.port, Some(13328));
    }

    #[test]
    fn the_signature_is_found_anywhere_in_a_segment() {
        assert!(contains_signature(&[0x01, 0x0E, 0x00, 0x36, 0x02]));
        assert!(contains_signature(&[0x06, 0x00, 0x36]));
        assert!(!contains_signature(&[0x0E, 0x00, 0x37]));
    }
}
