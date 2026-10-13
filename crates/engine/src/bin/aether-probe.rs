//! Find which game record carries a number you saw in game.
//!
//!   aether-probe capture.pcapng                       opcode counts
//!   aether-probe capture.pcapng --find 485 --find 445 frames holding these values
//!   aether-probe capture.pcapng --dump 2a38           every frame of one opcode, in hex
//!
//! Record the capture with Wireshark on the game PC while doing something whose
//! effect you know (a cube taking Odyle Energy from 485 to 445, a quest
//! turned in...). Opcodes that hold both the before and after values, at the
//! same offset, are the ones to decode.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use aether_engine::probe::{self, ProbeFrame};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next().map(PathBuf::from) else {
        eprintln!("usage: aether-probe <capture.pcapng> [--find <value>]... [--dump <opcode hex>]");
        return ExitCode::FAILURE;
    };
    let mut finds: Vec<u32> = Vec::new();
    let mut dump: Option<[u8; 2]> = None;
    while let Some(flag) = args.next() {
        match (flag.as_str(), args.next()) {
            ("--find", Some(v)) => match v.parse() {
                Ok(n) => finds.push(n),
                Err(_) => return fail(&format!("`{v}` is not a number")),
            },
            ("--dump", Some(op)) => match parse_opcode(&op) {
                Some(op) => dump = Some(op),
                None => return fail(&format!("`{op}` is not an opcode like 2a38")),
            },
            _ => return fail(&format!("unexpected `{flag}`")),
        }
    }

    let frames = match probe::read_frames(&path) {
        Ok(f) => f,
        Err(e) => return fail(&e),
    };
    println!("{} frames from {}", frames.len(), path.display());

    if let Some(op) = dump {
        for f in frames.iter().filter(|f| f.opcode == op) {
            println!("{:>9.3}s  {}", f.at_s, hex(&f.bytes));
        }
    } else if finds.is_empty() {
        let mut counts: BTreeMap<[u8; 2], usize> = BTreeMap::new();
        for f in &frames {
            *counts.entry(f.opcode).or_default() += 1;
        }
        let mut counts: Vec<_> = counts.into_iter().collect();
        counts.sort_by_key(|(_, n)| *n);
        for (op, n) in counts {
            println!("{}  {n}", hex(&op));
        }
    } else {
        report_finds(&frames, &finds);
    }
    ExitCode::SUCCESS
}

/// Each value's hits, then the opcodes that held every value at one offset.
fn report_finds(frames: &[ProbeFrame], values: &[u32]) {
    let mut by_place: BTreeMap<([u8; 2], usize, &'static str), Vec<u32>> = BTreeMap::new();
    for &value in values {
        let hits = probe::find(frames, value);
        println!("\n{value}: {} hit(s)", hits.len());
        for hit in hits.iter().take(40) {
            let f = &frames[hit.frame];
            println!(
                "  {:>9.3}s  {}  +{:<3} {:<6} {}",
                f.at_s,
                hex(&f.opcode),
                hit.offset,
                hit.encoding,
                hex(&f.bytes)
            );
            let found = by_place
                .entry((f.opcode, hit.offset, hit.encoding))
                .or_default();
            if !found.contains(&value) {
                found.push(value);
            }
        }
    }
    println!("\nopcode, offset and encoding holding every value:");
    let mut any = false;
    for ((op, offset, encoding), found) in &by_place {
        if found.len() == values.len() {
            any = true;
            println!("  {}  +{offset}  {encoding}", hex(op));
        }
    }
    if !any {
        println!("  none: try fewer values, or values read closer to the moment they changed");
    }
}

fn parse_opcode(text: &str) -> Option<[u8; 2]> {
    let digits: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if digits.len() != 4 {
        return None;
    }
    Some([
        u8::from_str_radix(&digits[..2], 16).ok()?,
        u8::from_str_radix(&digits[2..], 16).ok()?,
    ])
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn fail(message: &str) -> ExitCode {
    eprintln!("aether-probe: {message}");
    ExitCode::FAILURE
}
