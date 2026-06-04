//! `EspConstruct` — Layer 0 only, ESP32-class hardware.
//!
/// Fixed 256-entry lookup table, no heap, no OS.

use crate::types::{TritAction, BareMetalCapabilities, Query, Response, ConstructError, HardwareTier};
use crate::layer0::BareMetalConstruct;

/// ESP32-class bare-metal construct.
///
/// Uses a static 256-entry lookup table (one byte per entry). No heap allocation
/// anywhere in the query path.
pub struct EspConstruct {
    /// O(1) lookup table: index → TritAction (stored as u8).
    table: [u8; 256],
    caps: BareMetalCapabilities,
}

impl EspConstruct {
    /// Create a new `EspConstruct` with a default lookup table.
    ///
    /// The default table is seeded with a simple pattern:
    /// - indices 0–85: Avoid
    /// - indices 86–170: Explore
    /// - indices 171–255: Choose
    pub const fn new() -> Self {
        let mut table = [0u8; 256];
        let mut i = 0;
        while i < 256 {
            table[i] = if i < 86 {
                TritAction::Avoid.as_u8()
            } else if i < 171 {
                TritAction::Explore.as_u8()
            } else {
                TritAction::Choose.as_u8()
            };
            i += 1;
        }
        Self {
            table,
            caps: BareMetalCapabilities::new(
                256,
                false,  // no confidence scores on bare metal
                0x01,   // only Action query kind
                64,     // 64-byte max payload
            ),
        }
    }

    /// Create with a custom lookup table (4 entries, expanded to 256 via repetition).
    pub const fn with_pattern(a: TritAction, b: TritAction, c: TritAction, d: TritAction) -> Self {
        let pattern = [a.as_u8(), b.as_u8(), c.as_u8(), d.as_u8()];
        let mut table = [0u8; 256];
        let mut i = 0;
        while i < 256 {
            table[i] = pattern[i % 4];
            i += 1;
        }
        Self {
            table,
            caps: BareMetalCapabilities::new(256, false, 0x01, 64),
        }
    }

    /// Advisory hardware tier.
    pub const fn tier(&self) -> HardwareTier {
        HardwareTier::Embedded
    }
}

impl BareMetalConstruct for EspConstruct {
    fn query_lookup(&self, index: u16) -> TritAction {
        let idx = (index % 256) as usize;
        TritAction::from_u8(self.table[idx]).unwrap_or(TritAction::Explore)
    }

    fn capabilities(&self) -> BareMetalCapabilities {
        self.caps
    }

    fn query(&self, q: Query<'_>) -> Result<Response<'static>, ConstructError> {
        if q.payload.is_empty() {
            return Err(ConstructError::InvalidQuery);
        }
        if !self.caps.supports_query_kind(q.kind) {
            return Err(ConstructError::NotAvailable);
        }
        let idx = q.payload[0] as u16;
        let action = self.query_lookup(idx);
        Ok(Response::new(action, 1.0, &[]))
    }
}

impl Default for EspConstruct {
    fn default() -> Self {
        Self::new()
    }
}
