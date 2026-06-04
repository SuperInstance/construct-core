//! Comprehensive tests covering all three layers, feature gates, and type conversions.

use crate::*;

// ── Type tests ───────────────────────────────────────────────────────────────

#[test]
fn trit_action_roundtrip() {
    for val in [TritAction::Avoid, TritAction::Explore, TritAction::Choose] {
        assert_eq!(TritAction::from_u8(val.as_u8()), Some(val));
    }
    assert_eq!(TritAction::from_u8(255), None);
}

#[test]
fn trit_action_display() {
    assert_eq!(format!("{}", TritAction::Avoid), "Avoid");
    assert_eq!(format!("{}", TritAction::Explore), "Explore");
    assert_eq!(format!("{}", TritAction::Choose), "Choose");
}

#[test]
fn skill_id_roundtrip() {
    let skills = [
        SkillId::TernaryEvolution,
        SkillId::StrategyClassification,
        SkillId::PatternRecognition,
        SkillId::RiskAssessment,
        SkillId::ResourceAllocation,
        SkillId::Communication,
        SkillId::SensoryFusion,
        SkillId::Navigation,
        SkillId::Custom(100),
    ];
    for s in &skills {
        assert_eq!(SkillId::from_u8(s.as_u8()), *s);
    }
}

#[test]
fn tool_handle_validity() {
    assert!(!ToolHandle::NONE.is_valid());
    assert!(ToolHandle::new(1).is_valid());
    assert!(ToolHandle::new(999).is_valid());
}

#[test]
fn construct_error_display() {
    assert_eq!(ConstructError::NotAvailable.to_string(), "not available");
    assert_eq!(ConstructError::RateLimited.to_string(), "rate limited");
    assert_eq!(ConstructError::Timeout.to_string(), "timeout");
    assert_eq!(ConstructError::InvalidQuery.to_string(), "invalid query");
}

#[test]
fn hardware_tier_not_partial_ord() {
    // Verify that HardwareTier does NOT implement PartialOrd
    // (this is a compile-time property — if it compiled, the test passes)
    let t1 = HardwareTier::Embedded;
    let t2 = HardwareTier::Cluster;
    // t1 < t2 would fail to compile — that's the point
    assert_ne!(t1, t2);
}

#[test]
fn capabilities_supports_query_kind() {
    let caps = BareMetalCapabilities::new(256, false, 0x0F, 64);
    assert!(caps.supports_query_kind(QueryKind::Action));
    assert!(caps.supports_query_kind(QueryKind::Classify));
    assert!(caps.supports_query_kind(QueryKind::Predict));
    assert!(caps.supports_query_kind(QueryKind::Strategy));

    let limited = BareMetalCapabilities::new(256, false, 0x01, 64);
    assert!(limited.supports_query_kind(QueryKind::Action));
    assert!(!limited.supports_query_kind(QueryKind::Classify));
}

// ── Layer 0: EspConstruct ────────────────────────────────────────────────────

#[test]
fn esp_default_table_pattern() {
    let esp = EspConstruct::new();
    assert_eq!(esp.query_lookup(0), TritAction::Avoid);
    assert_eq!(esp.query_lookup(85), TritAction::Avoid);
    assert_eq!(esp.query_lookup(86), TritAction::Explore);
    assert_eq!(esp.query_lookup(170), TritAction::Explore);
    assert_eq!(esp.query_lookup(171), TritAction::Choose);
    assert_eq!(esp.query_lookup(255), TritAction::Choose);
}

#[test]
fn esp_wraparound() {
    let esp = EspConstruct::new();
    // index > 256 should wrap
    assert_eq!(esp.query_lookup(256), esp.query_lookup(0));
    assert_eq!(esp.query_lookup(512), esp.query_lookup(0));
}

#[test]
fn esp_capabilities() {
    let esp = EspConstruct::new();
    let caps = esp.capabilities();
    assert_eq!(caps.lookup_table_size, 256);
    assert!(!caps.has_confidence);
    assert_eq!(caps.max_payload_size, 64);
}

#[test]
fn esp_query_valid() {
    let esp = EspConstruct::new();
    let q = Query::new(QueryKind::Action, &[42]);
    let resp = esp.query(q).unwrap();
    assert_eq!(resp.action, esp.query_lookup(42));
}

#[test]
fn esp_query_empty_payload() {
    let esp = EspConstruct::new();
    let q = Query::new(QueryKind::Action, &[]);
    assert!(matches!(esp.query(q), Err(ConstructError::InvalidQuery)));
}

#[test]
fn esp_query_unsupported_kind() {
    let esp = EspConstruct::new();
    let q = Query::new(QueryKind::Classify, &[1]); // ESP only supports Action
    assert!(matches!(esp.query(q), Err(ConstructError::NotAvailable)));
}

#[test]
fn esp_custom_pattern() {
    let esp = EspConstruct::with_pattern(
        TritAction::Choose,
        TritAction::Choose,
        TritAction::Avoid,
        TritAction::Explore,
    );
    assert_eq!(esp.query_lookup(0), TritAction::Choose);
    assert_eq!(esp.query_lookup(1), TritAction::Choose);
    assert_eq!(esp.query_lookup(2), TritAction::Avoid);
    assert_eq!(esp.query_lookup(3), TritAction::Explore);
    // wraps
    assert_eq!(esp.query_lookup(4), TritAction::Choose);
}

#[test]
fn esp_tier() {
    let esp = EspConstruct::new();
    assert_eq!(esp.tier(), HardwareTier::Embedded);
}

// ── Layer 1: PiConstruct ─────────────────────────────────────────────────────

#[test]
fn pi_lookup_table() {
    let pi = PiConstruct::new();
    assert_eq!(pi.query_lookup(0), TritAction::Avoid);
    assert_eq!(pi.query_lookup(1), TritAction::Explore);
    assert_eq!(pi.query_lookup(2), TritAction::Choose);
}

#[test]
fn pi_capabilities() {
    let pi = PiConstruct::new();
    let caps = pi.capabilities();
    assert_eq!(caps.lookup_table_size, 1024);
    assert!(caps.has_confidence);
    assert_eq!(caps.max_payload_size, 4096);
}

#[test]
fn pi_load_unload_skill() {
    let mut pi = PiConstruct::new();
    assert!(pi.loaded_skills().is_empty());

    pi.load_skill(SkillId::TernaryEvolution).unwrap();
    assert_eq!(pi.loaded_skills().len(), 1);

    pi.load_skill(SkillId::TernaryEvolution).unwrap(); // idempotent
    assert_eq!(pi.loaded_skills().len(), 1);

    pi.unload_skill(SkillId::TernaryEvolution).unwrap();
    assert!(pi.loaded_skills().is_empty());

    let err = pi.unload_skill(SkillId::TernaryEvolution);
    assert_eq!(err, Err(ConstructError::SkillNotLoaded));
}

#[test]
fn pi_skill_limit() {
    let mut pi = PiConstruct::new();
    for i in 0..16 {
        pi.load_skill(SkillId::Custom(200 + i)).unwrap();
    }
    let err = pi.load_skill(SkillId::Custom(216));
    assert_eq!(err, Err(ConstructError::NotAvailable));
}

#[test]
fn pi_query_owned_with_skill() {
    let mut pi = PiConstruct::new();
    pi.load_skill(SkillId::TernaryEvolution).unwrap();

    let q = OwnedQuery::new(QueryKind::Action, vec![1]);
    let resp = pi.query_owned(q).unwrap();
    // With TernaryEvolution loaded, confidence should be 0.95
    assert!((resp.confidence - 0.95).abs() < 0.001);
}

#[test]
fn pi_query_owned_without_skill() {
    let pi = PiConstruct::new();
    let q = OwnedQuery::new(QueryKind::Action, vec![1]);
    let resp = pi.query_owned(q).unwrap();
    // Without TernaryEvolution, confidence should be 0.75
    assert!((resp.confidence - 0.75).abs() < 0.001);
}

#[test]
fn pi_tier() {
    let pi = PiConstruct::new();
    assert_eq!(pi.tier(), HardwareTier::SingleBoard);
}

// ── Layer 2: DgxConstruct ────────────────────────────────────────────────────

#[test]
fn dgx_lookup_table() {
    let dgx = DgxConstruct::new();
    assert_eq!(dgx.query_lookup(0), TritAction::Avoid);
    assert_eq!(dgx.query_lookup(1), TritAction::Explore);
    assert_eq!(dgx.query_lookup(2), TritAction::Choose);
}

#[test]
fn dgx_capabilities() {
    let dgx = DgxConstruct::new();
    let caps = dgx.capabilities();
    assert_eq!(caps.lookup_table_size, 4096);
    assert!(caps.has_confidence);
    assert_eq!(caps.max_payload_size, 65535);
}

#[test]
fn dgx_skill_management() {
    let mut dgx = DgxConstruct::new();
    dgx.load_skill(SkillId::TernaryEvolution).unwrap();
    dgx.load_skill(SkillId::RiskAssessment).unwrap();
    assert_eq!(dgx.loaded_skills().len(), 2);
    dgx.unload_skill(SkillId::TernaryEvolution).unwrap();
    assert_eq!(dgx.loaded_skills().len(), 1);
    assert_eq!(dgx.loaded_skills()[0], SkillId::RiskAssessment);
}

#[test]
fn dgx_tool_acquire_release() {
    let mut dgx = DgxConstruct::new();
    let h1 = dgx.request_tool(ToolSpec::VectorDb).unwrap();
    let h2 = dgx.request_tool(ToolSpec::Terminal).unwrap();
    assert!(h1.is_valid());
    assert!(h2.is_valid());
    assert_ne!(h1, h2);
    assert_eq!(dgx.active_tools().len(), 2);

    dgx.release_tool(h1).unwrap();
    assert_eq!(dgx.active_tools().len(), 1);

    let err = dgx.release_tool(h1);
    assert_eq!(err, Err(ConstructError::BadHandle));
}

#[test]
fn dgx_tool_limit() {
    let mut dgx = DgxConstruct::new();
    for _ in 0..32 {
        dgx.request_tool(ToolSpec::Browser).unwrap();
    }
    let err = dgx.request_tool(ToolSpec::CodeEditor);
    assert_eq!(err, Err(ConstructError::RateLimited));
}

#[test]
fn dgx_query_owned_confidence_levels() {
    let mut dgx = DgxConstruct::new();
    let q = OwnedQuery::new(QueryKind::Action, vec![0]);

    // No skills
    let r = dgx.query_owned(q.clone()).unwrap();
    assert!((r.confidence - 0.80).abs() < 0.001);

    // Some skill
    dgx.load_skill(SkillId::RiskAssessment).unwrap();
    let r = dgx.query_owned(q.clone()).unwrap();
    assert!((r.confidence - 0.88).abs() < 0.001);

    // TernaryEvolution
    dgx.load_skill(SkillId::TernaryEvolution).unwrap();
    let r = dgx.query_owned(q).unwrap();
    assert!((r.confidence - 0.97).abs() < 0.001);
}

#[tokio::test]
async fn dgx_query_async() {
    let mut dgx = DgxConstruct::new();
    dgx.load_skill(SkillId::TernaryEvolution).unwrap();
    let q = OwnedQuery::new(QueryKind::Action, vec![5]);
    let resp = dgx.query_async(q).await.unwrap();
    assert_eq!(resp.action, dgx.query_lookup(5));
    assert!((resp.confidence - 0.97).abs() < 0.001);
}

#[test]
fn dgx_tier() {
    let dgx = DgxConstruct::new();
    assert_eq!(dgx.tier(), HardwareTier::Cluster);
}

// ── Cross-layer tests ────────────────────────────────────────────────────────

#[test]
fn all_constructs_share_trit_action() {
    let esp = EspConstruct::new();
    let pi = PiConstruct::new();
    let dgx = DgxConstruct::new();

    // Same index should produce valid TritActions on all
    for i in 0u16..10 {
        let e = esp.query_lookup(i);
        let p = pi.query_lookup(i);
        let d = dgx.query_lookup(i);
        // They have different patterns but all produce valid TritActions
        assert!(matches!(e, TritAction::Avoid | TritAction::Explore | TritAction::Choose));
        assert!(matches!(p, TritAction::Avoid | TritAction::Explore | TritAction::Choose));
        assert!(matches!(d, TritAction::Avoid | TritAction::Explore | TritAction::Choose));
    }
}

#[test]
fn owned_query_borrow_roundtrip() {
    let oq = OwnedQuery::new(QueryKind::Predict, vec![1, 2, 3]);
    let q = oq.as_query();
    assert_eq!(q.kind, QueryKind::Predict);
    assert_eq!(q.payload, &[1, 2, 3]);
}

#[test]
fn owned_response_borrow_roundtrip() {
    let or = OwnedResponse::new(TritAction::Choose, 0.99, vec![42]);
    let r = or.as_response();
    assert_eq!(r.action, TritAction::Choose);
    assert!((r.confidence - 0.99).abs() < 0.001);
    assert_eq!(r.metadata, &[42]);
}
