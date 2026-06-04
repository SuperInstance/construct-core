//! `PiConstruct` — Layer 0 + Layer 1, Raspberry Pi-class hardware.
//!
//! Heap-allocated skill set, synchronous queries with confidence scores.

extern crate alloc;

use alloc::vec::Vec;
use crate::types::{
    TritAction, BareMetalCapabilities, SkillId, Query, Response,
    OwnedQuery, OwnedResponse, ConstructError, HardwareTier,
};
use crate::layer0::BareMetalConstruct;
use crate::layer1::SyncConstruct;

const PI_TABLE_SIZE: u16 = 1024;

/// Raspberry Pi-class construct. Implements Layer 0 + Layer 1.
pub struct PiConstruct {
    table: Vec<u8>,
    skills: Vec<SkillId>,
    caps: BareMetalCapabilities,
}

impl PiConstruct {
    /// Create a new `PiConstruct` with a default lookup table.
    pub fn new() -> Self {
        let mut table = Vec::with_capacity(PI_TABLE_SIZE as usize);
        for i in 0..PI_TABLE_SIZE {
            let action = match i % 3 {
                0 => TritAction::Avoid,
                1 => TritAction::Explore,
                _ => TritAction::Choose,
            };
            table.push(action.as_u8());
        }
        Self {
            table,
            skills: Vec::new(),
            caps: BareMetalCapabilities::new(
                PI_TABLE_SIZE,
                true,   // has confidence
                0x0F,   // all query kinds
                4096,   // 4KB max payload
            ),
        }
    }

    /// Advisory hardware tier.
    pub fn tier(&self) -> HardwareTier {
        HardwareTier::SingleBoard
    }
}

impl BareMetalConstruct for PiConstruct {
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
        Ok(Response::new(action, 0.85, &[]))
    }
}

impl SyncConstruct for PiConstruct {
    fn load_skill(&mut self, id: SkillId) -> Result<(), ConstructError> {
        if self.skills.contains(&id) {
            return Ok(()); // already loaded, idempotent
        }
        if self.skills.len() >= 16 {
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

        // Skills modify confidence
        let confidence = if self.skills.contains(&SkillId::TernaryEvolution) {
            0.95
        } else {
            0.75
        };

        let metadata = vec![action.as_u8(), q.kind as u8];
        Ok(OwnedResponse::new(action, confidence, metadata))
    }
}

impl Default for PiConstruct {
    fn default() -> Self {
        Self::new()
    }
}
