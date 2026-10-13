//! Packet discovery: find which frames carry a value you saw in game.
//!
//! Record a capture (Wireshark, on the game PC) while doing something whose
//! effect you know, such as opening a cube that takes your Odyle Energy from
//! 485 to 445. Every decoded frame of the game's stream is replayed here, and
//! the frames holding those numbers, as little-endian integers or varints,
//! point at the record that reports them. That is how a new decoder starts.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use a2tools::capture::captured_payload::CapturedPayload;
use a2tools::capture::frame_tap;
use a2tools::capture::stream_assembler::StreamAssembler;
use a2tools::capture::stream_processor::{StreamProcessor, read_varint};
use a2tools::combat::data_storage::DataStorage;
use a2tools::i18n::lookup::{NpcLookup, SkillLookup};
use parking_lot::Mutex;

use crate::capture::pcap::{Next, PcapLib, tcp_payload};

/// One decoded game frame, length varint included.
#[derive(Debug, Clone)]
pub struct ProbeFrame {
    /// Seconds since the first packet of the capture.
    pub at_s: f64,
    pub opcode: [u8; 2],
    pub bytes: Vec<u8>,
}

/// Where a value was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub frame: usize,
    pub offset: usize,
    pub encoding: &'static str,
}

/// Replay `path` and return every frame of the game's stream.
///
/// The game's connection is the server-to-client flow carrying the most
/// record terminators; TLS connections from the same port are left out.
pub fn read_frames(path: &Path) -> Result<Vec<ProbeFrame>, String> {
    let lib = PcapLib::load()?;
    let mut handle = lib.open_file(path)?;
    let link = handle.link_type();
    let mut packets: Vec<(i64, CapturedPayload)> = Vec::new();
    loop {
        match handle.next() {
            Next::Frame(frame, ts) => {
                if let Some(p) = tcp_payload(frame, link, "probe", ts) {
                    packets.push((ts, p));
                }
            }
            Next::Timeout => {}
            Next::End | Next::Error => break,
        }
    }
    let first_ts = packets.first().map_or(0, |(ts, _)| *ts);

    let mut score: HashMap<(u16, u16), usize> = HashMap::new();
    for (_, p) in &packets {
        if crate::dispatch::contains_signature(&p.data) {
            *score.entry((p.src_port, p.dst_port)).or_default() += 1;
        }
    }
    let flow = score
        .into_iter()
        .max_by_key(|(_, hits)| *hits)
        .map(|(flow, _)| flow)
        .ok_or("no game traffic in this capture (no record terminators found)")?;

    let frames = Arc::new(Mutex::new(Vec::new()));
    let clock = Arc::new(AtomicI64::new(first_ts));
    {
        let (frames, clock) = (frames.clone(), clock.clone());
        frame_tap::set(Some(Arc::new(move |frame: &[u8]| {
            let len = read_varint(frame, 0);
            let at = len.length.max(0) as usize;
            if let Some(op) = frame.get(at..at + 2) {
                let t = clock.load(Ordering::Relaxed);
                frames.lock().push(ProbeFrame {
                    at_s: (t - first_ts) as f64 / 1000.0,
                    opcode: [op[0], op[1]],
                    bytes: frame.to_vec(),
                });
            }
        })));
    }
    let storage = Arc::new(DataStorage::new());
    let mut processor = StreamProcessor::new(
        storage,
        Arc::new(SkillLookup::new()),
        Arc::new(NpcLookup::new()),
    );
    let mut assembler = StreamAssembler::new();
    for (ts, p) in packets
        .iter()
        .filter(|(_, p)| (p.src_port, p.dst_port) == flow)
    {
        clock.store(*ts, Ordering::Relaxed);
        assembler.process_chunk(&p.data, &mut processor);
    }
    frame_tap::set(None);

    let frames = std::mem::take(&mut *frames.lock());
    Ok(frames)
}

/// Frames holding `value` as a u16, u32 or varint.
pub fn find(frames: &[ProbeFrame], value: u32) -> Vec<Hit> {
    let mut patterns: Vec<(&'static str, Vec<u8>)> = vec![
        ("u32", value.to_le_bytes().to_vec()),
        ("varint", varint(value)),
    ];
    if let Ok(short) = u16::try_from(value) {
        patterns.push(("u16", short.to_le_bytes().to_vec()));
    }
    let mut hits = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        for (encoding, pattern) in &patterns {
            for (offset, window) in frame.bytes.windows(pattern.len()).enumerate() {
                if window == pattern.as_slice() {
                    hits.push(Hit {
                        frame: index,
                        offset,
                        encoding,
                    });
                }
            }
        }
    }
    hits
}

fn varint(mut value: u32) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(bytes: &[u8]) -> ProbeFrame {
        ProbeFrame {
            at_s: 0.0,
            opcode: [bytes[1], bytes[2]],
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn values_are_found_in_every_encoding() {
        let frames = vec![
            frame(&[0x09, 0x12, 0x34, 0xE5, 0x01, 0x00, 0x00]), // 485 as u32 (and u16)
            frame(&[0x06, 0x12, 0x35, 0xE5, 0x03]),             // 485 as a varint
            frame(&[0x05, 0x12, 0x36, 0x00]),
        ];
        let hits = find(&frames, 485);
        assert!(hits.contains(&Hit {
            frame: 0,
            offset: 3,
            encoding: "u32"
        }));
        assert!(hits.contains(&Hit {
            frame: 0,
            offset: 3,
            encoding: "u16"
        }));
        assert!(hits.contains(&Hit {
            frame: 1,
            offset: 3,
            encoding: "varint"
        }));
        assert!(hits.iter().all(|h| h.frame != 2));
    }
}
