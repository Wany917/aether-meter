//! libpcap through `libloading`: Npcap's `wpcap.dll` on Windows, the system
//! libpcap elsewhere. Loading at runtime means no SDK at build time and a clear
//! error (rather than a crash at launch) when Npcap is not installed.
//!
//! Adapted from A2Tools DPS Meter (GPL-3.0), `src-tauri/src/capture/pcap_capturer.rs`.

use std::ffi::{CStr, CString, c_char, c_int, c_long, c_uint, c_void};
use std::net::Ipv4Addr;
use std::ptr;

use a2tools::capture::captured_payload::CapturedPayload;
use libloading::Library;

type PcapT = *mut c_void;

#[repr(C)]
struct PcapIf {
    next: *mut PcapIf,
    name: *const c_char,
    description: *const c_char,
    addresses: *mut c_void,
    flags: c_uint,
}

/// `struct pcap_pkthdr`. `struct timeval` is two `long`s on Windows and Linux,
/// but a `long` and an `int` (padded) on macOS.
#[repr(C)]
struct PcapPkthdr {
    ts_sec: c_long,
    #[cfg(not(target_os = "macos"))]
    ts_usec: c_long,
    #[cfg(target_os = "macos")]
    ts_usec: i32,
    #[cfg(target_os = "macos")]
    _pad: i32,
    caplen: c_uint,
    len: c_uint,
}

const PCAP_IF_LOOPBACK: c_uint = 0x1;
const ERRBUF_SIZE: usize = 256;

#[cfg(windows)]
const LIBRARIES: &[&str] = &["wpcap.dll"];
#[cfg(target_os = "macos")]
const LIBRARIES: &[&str] = &["libpcap.dylib", "libpcap.A.dylib"];
#[cfg(all(unix, not(target_os = "macos")))]
const LIBRARIES: &[&str] = &["libpcap.so.1", "libpcap.so"];

#[cfg(windows)]
const MISSING_HELP: &str =
    "Install Npcap (https://npcap.com) with \"WinPcap API-compatible Mode\" checked.";
#[cfg(not(windows))]
const MISSING_HELP: &str = "Install libpcap.";

#[derive(Debug, Clone)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub is_loopback: bool,
    has_addresses: bool,
}

impl Device {
    pub fn label(&self) -> &str {
        if self.description.is_empty() {
            &self.name
        } else {
            &self.description
        }
    }

    /// Loopback, VPN and tunnel adapters. Game relays (ExitLag and the like)
    /// hand the game's traffic to one of these, so they are opened first.
    pub fn is_virtual(&self) -> bool {
        let label = self.label().to_lowercase();
        self.is_loopback
            || self.name.to_lowercase().contains("loopback")
            || ["loopback", "tap", "wintun", "wireguard"]
                .iter()
                .any(|k| label.contains(k))
    }
}

pub struct PcapLib {
    _lib: Library,
    findalldevs: unsafe extern "C" fn(*mut *mut PcapIf, *mut c_char) -> c_int,
    freealldevs: unsafe extern "C" fn(*mut PcapIf),
    open_live: unsafe extern "C" fn(*const c_char, c_int, c_int, c_int, *mut c_char) -> PcapT,
    open_offline: unsafe extern "C" fn(*const c_char, *mut c_char) -> PcapT,
    close: unsafe extern "C" fn(PcapT),
    next_ex: unsafe extern "C" fn(PcapT, *mut *mut PcapPkthdr, *mut *const u8) -> c_int,
    datalink: unsafe extern "C" fn(PcapT) -> c_int,
}

// SAFETY: the function pointers are plain C entry points; every thread opens
// and owns its own `pcap_t`, which is what libpcap requires.
unsafe impl Send for PcapLib {}
unsafe impl Sync for PcapLib {}

impl PcapLib {
    pub fn load() -> Result<Self, String> {
        prepare_search_path();
        let mut errors = Vec::new();
        let lib = LIBRARIES
            .iter()
            // SAFETY: loading libpcap runs no initialisation we depend on not running.
            .find_map(|name| match unsafe { Library::new(name) } {
                Ok(lib) => Some(lib),
                Err(e) => {
                    errors.push(format!("{name}: {e}"));
                    None
                }
            })
            .ok_or_else(|| format!("{} ({})", MISSING_HELP, errors.join("; ")))?;

        // SAFETY: the symbol types match libpcap's documented C signatures.
        unsafe {
            Ok(Self {
                findalldevs: *lib.get(b"pcap_findalldevs\0").map_err(|e| e.to_string())?,
                freealldevs: *lib.get(b"pcap_freealldevs\0").map_err(|e| e.to_string())?,
                open_live: *lib.get(b"pcap_open_live\0").map_err(|e| e.to_string())?,
                open_offline: *lib.get(b"pcap_open_offline\0").map_err(|e| e.to_string())?,
                close: *lib.get(b"pcap_close\0").map_err(|e| e.to_string())?,
                next_ex: *lib.get(b"pcap_next_ex\0").map_err(|e| e.to_string())?,
                datalink: *lib.get(b"pcap_datalink\0").map_err(|e| e.to_string())?,
                _lib: lib,
            })
        }
    }

    /// Devices worth opening: those with an address.
    pub fn devices(&self) -> Result<Vec<Device>, String> {
        let mut all: *mut PcapIf = ptr::null_mut();
        let mut errbuf = [0 as c_char; ERRBUF_SIZE];
        // SAFETY: both out-pointers are valid for the call.
        let ret = unsafe { (self.findalldevs)(&mut all, errbuf.as_mut_ptr()) };
        if ret != 0 {
            return Err(format!("pcap_findalldevs: {}", errbuf_text(&errbuf)));
        }

        let mut devices = Vec::new();
        let mut node = all;
        while !node.is_null() {
            // SAFETY: a node of the list pcap_findalldevs just returned.
            let d = unsafe { &*node };
            devices.push(Device {
                name: c_text(d.name),
                description: c_text(d.description),
                is_loopback: d.flags & PCAP_IF_LOOPBACK != 0,
                has_addresses: !d.addresses.is_null(),
            });
            node = d.next;
        }
        if !all.is_null() {
            // SAFETY: the list we own, freed once.
            unsafe { (self.freealldevs)(all) };
        }
        Ok(devices
            .into_iter()
            .filter(|d| d.has_addresses && d.name != "any")
            .collect())
    }

    pub fn open_live(&self, device: &str) -> Result<Handle<'_>, String> {
        let name = CString::new(device).map_err(|e| e.to_string())?;
        let mut errbuf = [0 as c_char; ERRBUF_SIZE];
        // Not promiscuous: only this machine's own traffic. 100 ms read timeout
        // so the loop notices a stop request.
        // SAFETY: valid C strings and buffer for the call.
        let raw = unsafe { (self.open_live)(name.as_ptr(), 65535, 0, 100, errbuf.as_mut_ptr()) };
        self.wrap(raw, &errbuf)
    }

    pub fn open_file(&self, path: &std::path::Path) -> Result<Handle<'_>, String> {
        let name = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
        let mut errbuf = [0 as c_char; ERRBUF_SIZE];
        // SAFETY: valid C strings and buffer for the call.
        let raw = unsafe { (self.open_offline)(name.as_ptr(), errbuf.as_mut_ptr()) };
        self.wrap(raw, &errbuf)
    }

    fn wrap(&self, raw: PcapT, errbuf: &[c_char]) -> Result<Handle<'_>, String> {
        if raw.is_null() {
            return Err(errbuf_text(errbuf));
        }
        // SAFETY: a live handle.
        let link_type = unsafe { (self.datalink)(raw) };
        Ok(Handle {
            lib: self,
            raw,
            link_type,
        })
    }
}

/// One open capture. Closed on drop.
pub struct Handle<'a> {
    lib: &'a PcapLib,
    raw: PcapT,
    link_type: c_int,
}

pub enum Next<'p> {
    /// A frame and its capture time in epoch ms.
    Frame(&'p [u8], i64),
    Timeout,
    End,
    Error,
}

impl Handle<'_> {
    pub fn link_type(&self) -> i32 {
        self.link_type
    }

    pub fn next(&mut self) -> Next<'_> {
        let mut header: *mut PcapPkthdr = ptr::null_mut();
        let mut data: *const u8 = ptr::null();
        // SAFETY: out-pointers valid for the call; the handle is ours.
        match unsafe { (self.lib.next_ex)(self.raw, &mut header, &mut data) } {
            1 => {
                // SAFETY: on 1, libpcap filled both pointers; the data lives
                // until the next call on this handle, which `&mut self` enforces.
                let (hdr, frame) = unsafe {
                    (
                        &*header,
                        std::slice::from_raw_parts(data, (*header).caplen as usize),
                    )
                };
                // `c_long` is i32 on Windows, i64 elsewhere.
                #[allow(clippy::useless_conversion)]
                let ts = i64::from(hdr.ts_sec) * 1000 + i64::from(hdr.ts_usec) / 1000;
                Next::Frame(frame, ts)
            }
            0 => Next::Timeout,
            -2 => Next::End,
            _ => Next::Error,
        }
    }
}

impl Drop for Handle<'_> {
    fn drop(&mut self) {
        // SAFETY: closed exactly once.
        unsafe { (self.lib.close)(self.raw) };
    }
}

#[cfg(windows)]
fn prepare_search_path() {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetDllDirectoryW(path: *const u16) -> i32;
    }

    // Without WinPcap-compatible mode, Npcap installs wpcap.dll in
    // System32\Npcap only, which a plain load does not search.
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let folder = std::path::Path::new(&root).join("System32").join("Npcap");
        let wide: Vec<u16> = folder
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: a NUL-terminated path that outlives the call.
        unsafe { SetDllDirectoryW(wide.as_ptr()) };
    });
}

#[cfg(not(windows))]
fn prepare_search_path() {}

fn c_text(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: a NUL-terminated string owned by libpcap for the list's lifetime.
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

fn errbuf_text(buf: &[c_char]) -> String {
    // SAFETY: libpcap NUL-terminates its error buffer; zero-initialised otherwise.
    unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

/// Where IPv4 starts in a frame of libpcap link type `link_type`.
fn link_header_len(link_type: i32, frame: &[u8]) -> Option<usize> {
    let be16 = |at: usize| {
        frame
            .get(at..at + 2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
    };
    match link_type {
        // Ethernet, with up to two 802.1Q / 802.1ad VLAN tags.
        1 => {
            let mut at = 12;
            for _ in 0..2 {
                match be16(at)? {
                    0x8100 | 0x88A8 => at += 4,
                    _ => break,
                }
            }
            (be16(at)? == 0x0800).then_some(at + 2)
        }
        // BSD / Npcap loopback: 4-byte address family, AF_INET = 2.
        0 => (frame.get(..4)? == [2, 0, 0, 0]).then_some(4),
        // Raw IP (VPN and tunnel adapters).
        12 | 14 | 101 | 228 => Some(0),
        // Linux cooked capture v1 and v2.
        113 => (be16(14)? == 0x0800).then_some(16),
        276 => (be16(0)? == 0x0800).then_some(20),
        _ => None,
    }
}

/// The frame-only guess, for adapters that report a link type they do not use.
fn guess_link_header_len(frame: &[u8]) -> Option<usize> {
    if frame.len() >= 14 && u16::from_be_bytes([frame[12], frame[13]]) == 0x0800 {
        Some(14)
    } else if frame.starts_with(&[2, 0, 0, 0]) {
        Some(4)
    } else if frame.first().is_some_and(|b| b >> 4 == 4) {
        Some(0)
    } else {
        None
    }
}

/// The TCP payload of an IPv4 frame, or `None` for anything else.
pub fn tcp_payload(
    frame: &[u8],
    link_type: i32,
    device: &str,
    captured_at_ms: i64,
) -> Option<CapturedPayload> {
    let ip_at = link_header_len(link_type, frame)
        .filter(|&at| frame.get(at).is_some_and(|b| b >> 4 == 4))
        .or_else(|| guess_link_header_len(frame))?;
    let ip = frame.get(ip_at..)?;
    if ip.len() < 20 || ip[0] >> 4 != 4 || ip[9] != 6 {
        return None;
    }
    let ip_header_len = usize::from(ip[0] & 0x0F) * 4;
    let tcp = ip.get(ip_header_len..)?;
    if tcp.len() < 20 {
        return None;
    }
    let tcp_header_len = usize::from(tcp[12] >> 4) * 4;

    // The payload ends where IP says, not at the frame's end: Ethernet pads
    // short frames, sometimes with non-zero bytes that would desync the game
    // stream. A total length of 0 (segmentation offload) keeps the frame's end.
    let ip_total = usize::from(u16::from_be_bytes([ip[2], ip[3]]));
    let end = if ip_total >= ip_header_len + 20 && ip_total <= ip.len() {
        ip_total
    } else {
        ip.len()
    };
    let start = ip_header_len + tcp_header_len;
    if start >= end {
        return None;
    }

    Some(CapturedPayload {
        src_port: u16::from_be_bytes([tcp[0], tcp[1]]),
        dst_port: u16::from_be_bytes([tcp[2], tcp[3]]),
        data: ip[start..end].to_vec(),
        device_name: Some(device.to_string()),
        captured_at_ms,
        src_ip: Some(Ipv4Addr::new(ip[12], ip[13], ip[14], ip[15])),
        dst_ip: Some(Ipv4Addr::new(ip[16], ip[17], ip[18], ip[19])),
        tcp_seq: u32::from_be_bytes([tcp[4], tcp[5], tcp[6], tcp[7]]),
        tcp_ack: u32::from_be_bytes([tcp[8], tcp[9], tcp[10], tcp[11]]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// IPv4 + TCP from 10.0.0.2:51000 to 193.202.112.99:13328, payload "hi".
    fn ipv4_tcp() -> Vec<u8> {
        let mut ip = vec![
            0x45, 0, 0, 42, 0, 0, 0, 0, 64, 6, 0, 0, 10, 0, 0, 2, 193, 202, 112, 99,
        ];
        ip.extend_from_slice(&[
            0xC7, 0x38, 0x34, 0x10, 0, 0, 0, 1, 0, 0, 0, 0, 0x50, 0x18, 0, 0, 0, 0, 0, 0,
        ]);
        ip.extend_from_slice(b"hi");
        ip
    }

    fn ethernet() -> Vec<u8> {
        [[0u8; 12].as_slice(), &[0x08, 0x00]].concat()
    }

    #[test]
    fn reads_ethernet_vlan_loopback_and_raw_ip() {
        let vlan = [[0u8; 12].as_slice(), &[0x81, 0x00, 0x00, 0x05, 0x08, 0x00]].concat();
        for (link, header) in [
            (1, ethernet()),
            (1, vlan),
            (0, vec![2, 0, 0, 0]),
            (12, vec![]),
        ] {
            let frame = [header.as_slice(), &ipv4_tcp()].concat();
            let p = tcp_payload(&frame, link, "dev", 0).expect("parsed");
            assert_eq!(
                (p.src_port, p.dst_port, p.data.as_slice()),
                (51000, 13328, b"hi".as_slice())
            );
        }
    }

    #[test]
    fn ethernet_padding_is_not_payload() {
        let mut ack = ipv4_tcp();
        ack.truncate(40);
        ack[3] = 40;
        let frame = [
            ethernet().as_slice(),
            &ack,
            &[0xbe, 0xde, 0x00, 0x03, 0x66, 0xfd],
        ]
        .concat();
        assert!(tcp_payload(&frame, 1, "dev", 0).is_none());
    }
}
