//! Layer 2 — `AsyncConstruct` trait.
//!
/// Full `std` + async runtime. Tool acquisition/release, async queries,
/// tokio-based runtime. Works on workstations, DGX clusters, cloud instances.

use crate::types::{ToolSpec, ToolHandle, OwnedQuery, OwnedResponse, ConstructError};
use crate::layer1::SyncConstruct;

/// An async construct with full std support and tool management.
///
/// Every `AsyncConstruct` is also a `SyncConstruct` (and transitively a
/// `BareMetalConstruct`). Layer 2 extends Layer 1 with async I/O and
/// tool acquisition semantics.
///
/// # Implementors
///
/// - `DgxConstruct` — DGX cluster-class hardware, all features enabled.
///
/// # Example
///
/// ```ignore
/// use construct_core::{AsyncConstruct, DgxConstruct, ToolSpec, QueryKind, OwnedQuery};
///
/// let mut dgx = DgxConstruct::new();
/// let handle = dgx.request_tool(ToolSpec::VectorDb).unwrap();
///
/// let q = OwnedQuery::new(QueryKind::Action, vec![1, 2, 3]);
/// let resp = dgx.query_async(q).await.unwrap();
///
/// dgx.release_tool(handle).unwrap();
/// ```
pub trait AsyncConstruct: SyncConstruct {
    /// Request a tool by specification. Returns a handle for tracking.
    ///
    /// The construct manages tool lifetimes — `release_tool` must be called
    /// when the tool is no longer needed.
    fn request_tool(&mut self, spec: ToolSpec) -> Result<ToolHandle, ConstructError>;

    /// Release a previously acquired tool.
    fn release_tool(&mut self, handle: ToolHandle) -> Result<(), ConstructError>;

    /// Return currently active tool handles.
    fn active_tools(&self) -> &[ToolHandle];

    /// Async query — the full-power interface. Can use acquired tools, async I/O,
    /// and the complete runtime.
    fn query_async(
        &self,
        q: OwnedQuery,
    ) -> impl std::future::Future<Output = Result<OwnedResponse, ConstructError>> + Send;
}
