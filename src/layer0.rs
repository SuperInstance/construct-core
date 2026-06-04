//! Layer 0 — `BareMetalConstruct` trait.
//!
//! The foundation. No heap, no async, no OS. Just fast O(1) table lookups
//! and static capability introspection. Works on ESP32, Cortex-M bare metal.

use crate::types::{TritAction, BareMetalCapabilities, Query, Response, ConstructError};

/// A bare-metal construct that can answer queries via static lookup tables.
///
/// This is the **only** layer available when compiled with `bare-metal` feature
/// (and no `alloc` / `std`). It uses `[u8; N]` arrays internally — no `Vec`,
/// `String`, or `Box`.
///
/// # Implementors
///
/// - `EspConstruct` — ESP32-class hardware, fixed 256-entry lookup table.
///
/// # Example
///
/// ```
/// use construct_core::{BareMetalConstruct, EspConstruct, TritAction};
///
/// let esp = EspConstruct::new();
/// let action = esp.query_lookup(42);
/// assert!(matches!(action, TritAction::Avoid | TritAction::Explore | TritAction::Choose));
/// ```
pub trait BareMetalConstruct {
    /// O(1) lookup into the internal action table.
    ///
    /// Returns the `TritAction` stored at `index`. Out-of-range indices wrap
    /// around using modular arithmetic — no panics, no `Option`.
    fn query_lookup(&self, index: u16) -> TritAction;

    /// Returns the static, compile-time-known capabilities of this construct.
    fn capabilities(&self) -> BareMetalCapabilities;

    /// Simple query interface for bare-metal. Returns a stack-allocated response.
    ///
    /// Default implementation uses `query_lookup` with the first byte of payload
    /// as the index. Override for smarter behavior.
    fn query(&self, q: Query<'_>) -> Result<Response<'static>, ConstructError> {
        let idx = q.payload.first().copied().unwrap_or(0) as u16;
        let action = self.query_lookup(idx);
        Ok(Response::new(action, 1.0, &[]))
    }
}
