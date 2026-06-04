//! # construct-core
//!
//! Hardware-agnostic agent runtime with a **three-layered trait system** for the
//! SuperInstance Construct API.
//!
//! ## Layers
//!
//! | Layer | Trait              | Environment          | Target Hardware          |
//! |-------|--------------------|----------------------|--------------------------|
//! | 0     | `BareMetalConstruct`| `no_std`, no alloc  | ESP32, bare Cortex-M     |
//! | 1     | `SyncConstruct`     | `no_std` + alloc    | Raspberry Pi, embedded Linux |
//! | 2     | `AsyncConstruct`    | `std` + async       | Workstation, DGX, Cloud  |
//!
//! Each higher layer *supersedes* the one below — a `DgxConstruct` implements all
//! three traits, while an `EspConstruct` only implements `BareMetalConstruct`.
//!
//! ## Feature Gates
//!
//! ```toml
//! [features]
//! default = ["std"]
//! std = ["alloc"]
//! alloc = []
//! bare-metal = []  # Only Layer 0
//! ```

#![cfg_attr(all(feature = "bare-metal", not(feature = "alloc")), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod types;
mod layer0;
mod layer1;
mod layer2;

pub use types::*;
pub use layer0::BareMetalConstruct;
#[cfg(feature = "alloc")]
pub use layer1::SyncConstruct;
#[cfg(feature = "std")]
pub use layer2::AsyncConstruct;

// ── Implementations ──────────────────────────────────────────────────────────

#[cfg(any(feature = "bare-metal", feature = "alloc", feature = "std"))]
mod esp;
#[cfg(feature = "alloc")]
mod pi;
#[cfg(feature = "std")]
mod dgx;

#[cfg(any(feature = "bare-metal", feature = "alloc", feature = "std"))]
pub use esp::EspConstruct;
#[cfg(feature = "alloc")]
pub use pi::PiConstruct;
#[cfg(feature = "std")]
pub use dgx::DgxConstruct;

#[cfg(test)]
mod tests;
