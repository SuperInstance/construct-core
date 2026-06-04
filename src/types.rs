//! Core types shared across all layers.
//!
//! Every type here is `no_std`-compatible. Heap types (`Vec`, `String`) are
//! gated behind `#[cfg(feature = "alloc")]`.

// ── TritAction ───────────────────────────────────────────────────────────────

/// The trinary action model — the fundamental decision output of every construct.
///
/// Agents never produce a simple yes/no; they choose between three actions
/// derived from trinary logic (balanced ternary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TritAction {
    /// Negative — avoid, reject, or retreat.
    Avoid  = 0,
    /// Zero / neutral — explore, gather more info, defer.
    Explore = 1,
    /// Positive — choose, commit, or execute.
    Choose = 2,
}

impl TritAction {
    /// Convert a `u8` to a `TritAction`. Returns `None` for invalid values.
    #[inline]
    pub const fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Avoid),
            1 => Some(Self::Explore),
            2 => Some(Self::Choose),
            _ => None,
        }
    }

    /// The raw `u8` representation.
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl core::fmt::Display for TritAction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Avoid   => write!(f, "Avoid"),
            Self::Explore => write!(f, "Explore"),
            Self::Choose  => write!(f, "Choose"),
        }
    }
}

// ── SkillId ──────────────────────────────────────────────────────────────────

/// Known skills that can be loaded into a construct.
///
/// Uses an enum instead of strings to keep things `no_std`-friendly and
/// avoid heap allocation on lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SkillId {
    TernaryEvolution     = 0,
    StrategyClassification = 1,
    PatternRecognition   = 2,
    RiskAssessment       = 3,
    ResourceAllocation   = 4,
    Communication        = 5,
    SensoryFusion        = 6,
    Navigation           = 7,
    /// Reserved for runtime-defined skills.
    Custom(u8),
}

impl SkillId {
    pub const fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::TernaryEvolution,
            1 => Self::StrategyClassification,
            2 => Self::PatternRecognition,
            3 => Self::RiskAssessment,
            4 => Self::ResourceAllocation,
            5 => Self::Communication,
            6 => Self::SensoryFusion,
            7 => Self::Navigation,
            other => Self::Custom(other),
        }
    }

    pub const fn as_u8(self) -> u8 {
        match self {
            Self::TernaryEvolution => 0,
            Self::StrategyClassification => 1,
            Self::PatternRecognition => 2,
            Self::RiskAssessment => 3,
            Self::ResourceAllocation => 4,
            Self::Communication => 5,
            Self::SensoryFusion => 6,
            Self::Navigation => 7,
            Self::Custom(v) => v,
        }
    }
}

// ── Query / QueryKind ────────────────────────────────────────────────────────

/// What kind of query is being asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum QueryKind {
    /// Ask for an action recommendation.
    Action     = 0,
    /// Ask for a classification / label.
    Classify   = 1,
    /// Ask for a numeric prediction.
    Predict    = 2,
    /// Ask for a strategy recommendation.
    Strategy   = 3,
}

/// A query to the construct. Uses byte slices for zero-copy where possible.
#[derive(Debug, Clone)]
pub struct Query<'a> {
    pub kind: QueryKind,
    pub payload: &'a [u8],
}

impl<'a> Query<'a> {
    pub const fn new(kind: QueryKind, payload: &'a [u8]) -> Self {
        Self { kind, payload }
    }
}

/// A response from the construct.
#[derive(Debug, Clone)]
pub struct Response<'a> {
    pub action: TritAction,
    pub confidence: f32,
    pub metadata: &'a [u8],
}

impl<'a> Response<'a> {
    pub const fn new(action: TritAction, confidence: f32, metadata: &'a [u8]) -> Self {
        Self { action, confidence, metadata }
    }
}

// ── Owned variants (alloc) ───────────────────────────────────────────────────

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// An owned query (heap version).
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct OwnedQuery {
    pub kind: QueryKind,
    pub payload: Vec<u8>,
}

#[cfg(feature = "alloc")]
impl OwnedQuery {
    pub fn new(kind: QueryKind, payload: Vec<u8>) -> Self {
        Self { kind, payload }
    }

    /// Borrow as a zero-copy `Query`.
    pub fn as_query(&self) -> Query<'_> {
        Query::new(self.kind, &self.payload)
    }
}

/// An owned response (heap version).
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct OwnedResponse {
    pub action: TritAction,
    pub confidence: f32,
    pub metadata: Vec<u8>,
}

#[cfg(feature = "alloc")]
impl OwnedResponse {
    pub fn new(action: TritAction, confidence: f32, metadata: Vec<u8>) -> Self {
        Self { action, confidence, metadata }
    }

    /// Borrow as a zero-copy `Response`.
    pub fn as_response(&self) -> Response<'_> {
        Response::new(self.action, self.confidence, &self.metadata)
    }
}

// ── ToolSpec / ToolHandle ────────────────────────────────────────────────────

/// Specification of a tool that can be requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ToolSpec {
    VectorDb        = 0,
    CodeEditor      = 1,
    Terminal        = 2,
    Browser         = 3,
    MotorController = 4,
}

/// A handle to a tool acquired by the construct. Just a `u32` ID — no trait objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToolHandle(pub u32);

impl ToolHandle {
    pub const NONE: Self = Self(0);

    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

// ── ConstructError ───────────────────────────────────────────────────────────

/// Errors that can occur during construct operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ConstructError {
    /// The requested operation or resource is not available on this hardware.
    NotAvailable  = 0,
    /// Rate limit exceeded.
    RateLimited   = 1,
    /// Operation timed out.
    Timeout       = 2,
    /// The query was malformed or invalid.
    InvalidQuery  = 3,
    /// The requested skill is not loaded.
    SkillNotLoaded = 4,
    /// The tool handle is invalid or already released.
    BadHandle     = 5,
}

impl core::fmt::Display for ConstructError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotAvailable   => write!(f, "not available"),
            Self::RateLimited    => write!(f, "rate limited"),
            Self::Timeout        => write!(f, "timeout"),
            Self::InvalidQuery   => write!(f, "invalid query"),
            Self::SkillNotLoaded => write!(f, "skill not loaded"),
            Self::BadHandle      => write!(f, "bad handle"),
        }
    }
}

// ── HardwareTier ─────────────────────────────────────────────────────────────

/// Advisory hardware tier. **No `PartialOrd`** — a tier is a category, not a
/// linear scale. A DGX isn't "greater than" an ESP32; it's a different class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HardwareTier {
    /// Microcontroller — no heap, no OS.
    Embedded    = 0,
    /// Single-board computer — heap, OS, no async runtime.
    SingleBoard = 1,
    /// Workstation or server — full std + async.
    Workstation = 2,
    /// Multi-GPU cluster — massive parallelism.
    Cluster     = 3,
}

impl core::fmt::Display for HardwareTier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Embedded    => write!(f, "Embedded"),
            Self::SingleBoard => write!(f, "SingleBoard"),
            Self::Workstation => write!(f, "Workstation"),
            Self::Cluster     => write!(f, "Cluster"),
        }
    }
}

// ── BareMetalCapabilities ────────────────────────────────────────────────────

/// Static capabilities of a bare-metal construct. All fields are const-compatible
/// and heap-free — just booleans and small integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BareMetalCapabilities {
    /// Number of action lookup table entries.
    pub lookup_table_size: u16,
    /// Whether the construct can provide confidence scores.
    pub has_confidence: bool,
    /// Number of supported query kinds (bitfield: bit 0=Action, 1=Classify, etc.).
    pub supported_query_kinds: u8,
    /// Maximum payload size in bytes.
    pub max_payload_size: u16,
}

impl BareMetalCapabilities {
    pub const fn new(
        lookup_table_size: u16,
        has_confidence: bool,
        supported_query_kinds: u8,
        max_payload_size: u16,
    ) -> Self {
        Self {
            lookup_table_size,
            has_confidence,
            supported_query_kinds,
            max_payload_size,
        }
    }

    /// Check if a specific query kind is supported.
    pub const fn supports_query_kind(&self, kind: QueryKind) -> bool {
        let bit = 1u8 << (kind as u8);
        (self.supported_query_kinds & bit) != 0
    }
}
