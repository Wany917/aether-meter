//! Odyle energy, as the game last sent it: the parser reads it from the
//! game's list of entry counts, at login and whenever it changes.

/// Both pools of Odyle energy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OdyleEnergy {
    /// Recharged every cycle up to the cap, and spent first.
    pub basic: u32,
    /// From recharge items, outside the cap.
    pub charged: u32,
}
