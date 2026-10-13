//! Pushes snapshots to the overlays at a steady rate.

use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::events::{MeterUpdated, TrackerUpdated};
use crate::state::AppState;
use crate::windows::{self, WindowKind};

const METER_EVERY: Duration = Duration::from_millis(300);
const TRACKER_EVERY: Duration = Duration::from_millis(250);

pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("tick".into())
        .spawn(move || {
            let mut last_meter = Instant::now() - METER_EVERY;
            let mut last_tracker = Instant::now() - TRACKER_EVERY;
            loop {
                std::thread::sleep(Duration::from_millis(50));
                let Some(state) = app.try_state::<AppState>() else {
                    continue;
                };

                if last_meter.elapsed() >= METER_EVERY {
                    last_meter = Instant::now();
                    // The hub's details read the meter's target, so keep
                    // computing while either is open.
                    if windows::is_visible(&app, WindowKind::Meter)
                        || windows::is_visible(&app, WindowKind::Hub)
                    {
                        let _ = MeterUpdated(state.engine.meter()).emit(&app);
                    }
                }
                if last_tracker.elapsed() >= TRACKER_EVERY {
                    last_tracker = Instant::now();
                    if windows::is_visible(&app, WindowKind::Tracker) {
                        let _ = TrackerUpdated(state.engine.tracker()).emit(&app);
                    }
                }
            }
        })
        .expect("spawn tick thread");
}
