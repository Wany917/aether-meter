//! What survives a failure in a release build, which has no console: a log
//! file, panics written to it, and a message box when the app cannot start.
//!
//! The log lives where players can find it:
//! Windows `%LOCALAPPDATA%\Aether\logs\aether.log`, macOS
//! `~/Library/Logs/Aether/aether.log`, Linux `~/.local/state/aether/aether.log`.

use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

/// Start a fresh file once it passes this size.
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

static LOG_FILE: OnceLock<PathBuf> = OnceLock::new();

fn log_dir() -> PathBuf {
    let home = || std::env::var_os("HOME").map(PathBuf::from);
    let dir = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Aether").join("logs"))
    } else if cfg!(target_os = "macos") {
        home().map(|h| h.join("Library").join("Logs").join("Aether"))
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| home().map(|h| h.join(".local").join("state")))
            .map(|d| d.join("aether"))
    };
    dir.unwrap_or_else(|| std::env::temp_dir().join("aether-logs"))
}

/// Log to stdout and to the log file, and record panics. Call first thing.
pub fn init() {
    let filter =
        || EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,aether=debug".into());
    let dir = log_dir();
    let path = dir.join("aether.log");
    let file = fs::create_dir_all(&dir).ok().and_then(|()| {
        if fs::metadata(&path).is_ok_and(|m| m.len() > MAX_LOG_BYTES) {
            let _ = fs::rename(&path, dir.join("aether.old.log"));
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok()
    });

    let registry = tracing_subscriber::registry()
        .with(filter())
        .with(fmt::layer());
    match file {
        Some(file) => {
            let _ = LOG_FILE.set(path);
            registry
                .with(fmt::layer().with_ansi(false).with_writer(Mutex::new(file)))
                .init();
        }
        None => registry.init(),
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("unnamed");
        tracing::error!("panic on thread `{name}`: {info}");
        if name == "main" {
            fatal(&info.to_string());
        }
        default_hook(info);
    }));

    tracing::info!(
        "Aether {} on {} {}, log at {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        LOG_FILE
            .get()
            .map_or_else(|| "(none)".into(), |p| p.display().to_string())
    );
}

/// The app cannot go on: say so, with the way to the log.
pub fn fatal(message: &str) {
    tracing::error!("fatal: {message}");
    let log = LOG_FILE
        .get()
        .map_or_else(String::new, |p| p.display().to_string());
    let text = format!(
        "Aether n'a pas pu démarrer.\nAether could not start.\n\n{message}\n\nJournal / Log:\n{log}"
    );
    show_error(&text);
}

#[cfg(windows)]
fn show_error(text: &str) {
    use std::ffi::c_void;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(
            window: *mut c_void,
            text: *const u16,
            caption: *const u16,
            kind: u32,
        ) -> i32;
    }
    const MB_ICONERROR: u32 = 0x10;
    let wide = |s: &str| {
        s.encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
    };
    let (text, caption) = (wide(text), wide("Aether"));
    // SAFETY: NUL-terminated UTF-16 strings that outlive the call; no owner window.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_ICONERROR,
        )
    };
}

#[cfg(not(windows))]
fn show_error(text: &str) {
    eprintln!("{text}");
}
