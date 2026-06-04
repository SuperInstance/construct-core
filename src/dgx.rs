//! `DgxConstruct` — All three layers, DGX cluster-class hardware.
//!
//! Full std + async runtime, tool management, maximum capability.

use alloc::vec::Vec;

use crate::types::{
    TritAction, BareMetalCapabilities, SkillId, Query, Response,
    OwnedQuery, OwnedResponse, ConstructError, HardwareTier,
    ToolSpec, ToolHandle,
};
use crate::layer0::BareMetalConstruct;
use crate::layer1::SyncConstruct;
use crate::layer2::AsyncConstruct;

const DGX_TABLE_SIZE: u16 = 4096;

/// DGX cluster-class construct. Implements all three layers.
pub struct DgxConstruct {
    table: Vec<u8>,
    skills: Vec<SkillId>,
    tools: Vec<ToolHandle>,
    next_tool_id: u32,
    caps: BareMetalCapabilities,
}

impl DgxConstruct {
    /// Create a new `DgxConstruct`.
    pub fn new() -> Self {
        let mut table = Vec::with_capacity(DGX_TABLE_SIZE as usize);
        for i in 0..DGX_TABLE_SIZE {
            let action = match i % 5 {
                0 => TritAction::Avoid,
                1 | 3 => TritAction::Explore,
                _ => TritAction::Choose,
            };
            table.push(action.as_u8());
        }
        Self {
            table,
            skills: Vec::new(),
            tools: Vec::new(),
            next_tool_id: 1,
            caps: BareMetalCapabilities::new(
                DGX_TABLE_SIZE,
                true,   // has confidence
                0x0F,   // all query kinds
                65535,  // 64KB max payload
            ),
        }
    }

    /// Advisory hardware tier.
    pub fn tier(&self) -> HardwareTier {
        HardwareTier::Cluster
    }
}

impl BareMetalConstruct for DgxConstruct {
    fn query_lookup(&self, index: u16) -> TritAction {
        let idx = (index as usize) % self.table.len();
        TritAction::from_u8(self.table[idx]).unwrap_or(TritAction::Explore)
    }

    fn capabilities(&self) -> BareMetalCapabilities {
        self.caps
    }

    fn query(&self, q: Query<'_>) -> Result<Response<'static>, ConstructError> {
        if !self.caps.supports_query_kind(q.kind) {
            return Err(ConstructError::NotAvailable);
        }
        let idx = q.payload.first().copied().unwrap_or(0) as u16;
        let action = self.query_lookup(idx);
        Ok(Response::new(action, 0.9, &[]))
    }
}

impl SyncConstruct for DgxConstruct {
    fn load_skill(&mut self, id: SkillId) -> Result<(), ConstructError> {
        if self.skills.contains(&id) {
            return Ok(());
        }
        if self.skills.len() >= 64 {
            return Err(ConstructError::NotAvailable);
        }
        self.skills.push(id);
        Ok(())
    }

    fn unload_skill(&mut self, id: SkillId) -> Result<(), ConstructError> {
        let pos = self.skills.iter().position(|&s| s == id);
        match pos {
            Some(i) => {
                self.skills.swap_remove(i);
                Ok(())
            }
            None => Err(ConstructError::SkillNotLoaded),
        }
    }

    fn loaded_skills(&self) -> &[SkillId] {
        &self.skills
    }

    fn query_owned(&self, q: OwnedQuery) -> Result<OwnedResponse, ConstructError> {
        if !self.caps.supports_query_kind(q.kind) {
            return Err(ConstructError::NotAvailable);
        }
        let idx = q.payload.first().copied().unwrap_or(0) as u16;
        let action = self.query_lookup(idx);

        let confidence = if self.skills.contains(&SkillId::TernaryEvolution) {
            0.97
        } else if !self.skills.is_empty() {
            0.88
        } else {
            0.80
        };

        let mut metadata = vec![action.as_u8(), q.kind as u8];
        metadata.extend_from_slice(&self.tools.len().to_le_bytes());

        Ok(OwnedResponse::new(action, confidence, metadata))
    }
}

impl AsyncConstruct for DgxConstruct {
    fn request_tool(&mut self, spec: ToolSpec) -> Result<ToolHandle, ConstructError> {
        if self.tools.len() >= 32 {
            return Err(ConstructError::RateLimited);
        }
        let handle = ToolHandle::new(self.next_tool_id);
        self.next_tool_id += 1;
        self.tools.push(handle);
        // spec is advisory; we don't differentiate at this level
        let _ = spec;
        Ok(handle)
    }

    fn release_tool(&mut self, handle: ToolHandle) -> Result<(), ConstructError> {
        let pos = self.tools.iter().position(|&h| h == handle);
        match pos {
            Some(i) => {
                self.tools.swap_remove(i);
                Ok(())
            }
            None => Err(ConstructError::BadHandle),
        }
    }

    fn active_tools(&self) -> &[ToolHandle] {
        &self.tools
    }

    async fn query_async(&self, q: OwnedQuery) -> Result<OwnedResponse, ConstructError> {
        // Simulate async I/O — in production this would hit GPU / network
        // Simulate async I/O — in production this would hit GPU / network
        // We use tokio only when available
        #[cfg(feature = "std")]
        tokio::time::sleep(std::time::Duration::from_micros(10)).await;
        self.query_owned(q)
    }
}

impl Default for DgxConstruct {
    fn default() -> Self {
        Self::new()
    }
}
