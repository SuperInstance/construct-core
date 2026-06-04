//! Layer 1 — `SyncConstruct` trait.
//!
//! Adds a heap (`alloc::vec::Vec`, `alloc::string::String`) but still no async.
//! Constructs at this layer can load/unload skills dynamically and answer more
//! sophisticated queries. Works on Raspberry Pi, embedded Linux.

use crate::types::{SkillId, OwnedQuery, OwnedResponse, ConstructError};
use crate::layer0::BareMetalConstruct;

/// A synchronous construct with heap allocation and skill management.
///
/// Every `SyncConstruct` is also a `BareMetalConstruct` — Layer 1 *extends*
/// Layer 0, it doesn't replace it.
///
/// # Implementors
///
/// - `PiConstruct` — Raspberry Pi-class hardware, dynamic skill loading.
///
/// # Example
///
/// ```ignore
/// use construct_core::{SyncConstruct, PiConstruct, SkillId, QueryKind, OwnedQuery};
///
/// let mut pi = PiConstruct::new();
/// pi.load_skill(SkillId::TernaryEvolution).unwrap();
///
/// let q = OwnedQuery::new(QueryKind::Action, vec![42]);
/// let resp = pi.query_owned(q).unwrap();
/// println!("action={}, confidence={}", resp.action, resp.confidence);
/// ```
pub trait SyncConstruct: BareMetalConstruct {
    /// Load a skill into the construct's active skill set.
    fn load_skill(&mut self, id: SkillId) -> Result<(), ConstructError>;

    /// Unload a skill from the construct's active skill set.
    fn unload_skill(&mut self, id: SkillId) -> Result<(), ConstructError>;

    /// Return the set of currently loaded skills.
    fn loaded_skills(&self) -> &[SkillId];

    /// Query the construct using an owned query, returning an owned response.
    ///
    /// This is the "real" query interface for Layer 1 — it can consult loaded
    /// skills and produce heap-allocated metadata.
    fn query_owned(&self, q: OwnedQuery) -> Result<OwnedResponse, ConstructError>;

    /// Convenience: borrow an owned query and delegate to `query_owned`.
    fn query_borrowed(&self, q: &OwnedQuery) -> Result<OwnedResponse, ConstructError> {
        self.query_owned(q.clone())
    }
}
