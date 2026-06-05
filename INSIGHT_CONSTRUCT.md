# Construct System Analysis

> How construct-core's trait system integrates with oxide-constructs for
> git-native GPU capability loading in the Flux→PTX runtime.

---

# Construct Systems Scout Analysis

> Comprehensive study of the SuperInstance construct system, git-native agents, and all runtime repos.
> Date: 2026-06-05
> Repos: 12 cloned and analyzed

---

## Table of Contents

1. [oxide-constructs](#oxide-constructs)
2. [construct-core](#construct-core)
3. [construct-coordination](#construct-coordination)
4. [cocapn-runtime](#cocapn-runtime)
5. [lever-runner + fastloop-guard](#lever-runner--fastloop-guard)
6. [plato-runtime-kernel](#plato-runtime-kernel)
7. [ternary-esp32-firmware](#ternary-esp32-firmware)
8. [oxide-flux-runtime](#oxide-flux-runtime)
9. [cudaclaw-bridge](#cudaclaw-bridge)
10. [intelligent-terminal](#intelligent-terminal)
11. [open-parallel](#open-parallel)
12. [The Construct Loading Protocol](#the-construct-loading-protocol)
13. [Constructs as GPU Primitives](#constructs-as-gpu-primitives)
14. [Edge↔GPU Communication Patterns](#edgegpu-communication-patterns)
15. [System Synthesis](#system-synthesis)

---

## oxide-constructs

**Repo:** `https://github.com/SuperInstance/oxide-constructs`  
**Language:** Rust  
**Lines:** ~608 (src/lib.rs)  
**License:** Apache-2.0

### What It Is

`oxide-constructs` is the **git-native construct loader** for the Flux→PTX distributed GPU runtime. It defines what a "construct" is, how it moves through lifecycle states, and how registries merge across a distributed fleet.

A **construct** is a self-contained unit of GPU capability living in a git repo. There are three kinds:

| Kind | Description | Example |
|------|-------------|---------|
| `Skill` | Software capability (kernels, shaders) | A ternary attention kernel |
| `Equipment` | Hardware requirement/advertisement | SM version, VRAM, tensor cores |
| `Hybrid` | Skill + equipment bundled together | A kernel with known hardware needs |

### Manifest Format: CONSTRUCT.toml

The manifest (specified in README, stubbed in code) contains:
- `name`, `version` — SemVer
- `construct` — type tag (skill/equipment/hybrid)
- `equipment` — min SM version, min VRAM, tensor core requirements
- `dependencies` — other constructs by git repo + version + symbol
- `identity` — DID-based creator fingerprint + signature
- `compute` — compute capability array (e.g., `[80, 86, 89, 90]`)

### Construct Lifecycle State Machine

```
Discovered → Validated → Resolved → Compiled → Deployed → Cached
                ↑                                    |
                └──────────── Failed ←───────────────┘
```

| State | Meaning |
|-------|---------|
| `Discovered` | Repo URL known, manifest not yet parsed |
| `Validated` | Manifest parsed, name non-empty, compute caps present |
| `Resolved` | Dependencies located (stubbed) |
| `Compiled` | PTX emitted, cached as placeholder |
| `Deployed` | Kernel live on GPU |
| `Cached` | Unloaded but retains compiled artifact |
| `Failed(String)` | Terminal error state |

### Registry: CRDT Merge

The `ConstructRegistry` is a HashMap-backed in-memory store with **last-write-wins CRDT semantics**:

```rust
pub fn merge(&mut self, remote: &ConstructRegistry) {
    for (id, remote_construct) in &remote.constructs {
        match self.constructs.get(id) {
            Some(local) if remote_construct.manifest.version > local.manifest.version => {
                self.constructs.insert(id.clone(), remote_construct.clone());
            }
            None => { self.constructs.insert(id.clone(), remote_construct.clone()); }
            _ => {} // local >= remote, keep local
        }
    }
}
```

This enables fleet-wide sync without a central server. Each node merges registries from peers; higher SemVer wins.

### Identity Verification

Constructs carry `ConstructIdentity` with DID, creator fingerprint, and signature. Currently stubbed — the loader sets `verify_identity: true` but does not actually verify signatures.

### Current State Assessment

**Skeleton/placeholder crate.** The data structures and state machine are well-designed, but actual git cloning, TOML parsing, dependency resolution, PTX compilation, and identity verification are all stubs. It acts as the **contract layer**.

---

## construct-core

**Repo:** `https://github.com/SuperInstance/construct-core`  
**Language:** Rust  
**Lines:** ~1,100  
**License:** MIT

### What It Is

The **v2 layered trait system** for the SuperInstance Construct API — a hardware-agnostic agent runtime. It replaces the v1 "fantasy specification" with a rigorous, `no_std`-compatible trait hierarchy.

### Three Layered Traits

```
Layer 2: AsyncConstruct        std + async (tokio)
  • request_tool / release_tool  Tool lifecycle management
  • query_async                  Async I/O, GPU, network
  • active_tools

Layer 1: SyncConstruct         no_std + alloc
  • load_skill / unload_skill    Dynamic skill management
  • query_owned                  Heap-allocated queries/responses
  • loaded_skills

Layer 0: BareMetalConstruct    no_std, no alloc
  • query_lookup                 O(1) table lookup
  • capabilities                 Static capability introspection
  • query (default impl)         Stack-only, zero alloc
```

### Feature Gates

| Feature | Layers | Target Hardware |
|---------|--------|----------------|
| `bare-metal` | 0 only | ESP32, Cortex-M |
| `alloc` | 0 + 1 | Raspberry Pi, Jetson |
| `std` (default) | 0 + 1 + 2 | Workstation, DGX, Cloud |

### Core Types (all no_std-compatible)

| Type | Description |
|------|-------------|
| `TritAction` | `Avoid=0`, `Explore=1`, `Choose=2` |
| `SkillId` | Enum of 8 known skills + `Custom(u8)` — no heap strings |
| `Query` / `OwnedQuery` | Zero-copy and heap-allocated query types |
| `ToolSpec` / `ToolHandle` | Tool request system, `ToolHandle` is `u32` not `Box<dyn>` |
| `HardwareTier` | Embedded, SingleBoard, Workstation, Cluster — no `PartialOrd` |

### Three Implementations

| Construct | Layer 0 | Layer 1 | Layer 2 | Table Size | Skills Max | Tools Max |
|-----------|---------|---------|---------|------------|------------|-----------|
| `EspConstruct` | Yes | No | No | 256-entry `[u8; 256]` | — | — |
| `PiConstruct` | Yes | Yes | No | 1024-entry `Vec<u8>` | 16 | — |
| `DgxConstruct` | Yes | Yes | Yes | 4096-entry `Vec<u8>` | 64 | 32 |

### Design Philosophy

> "Because hardware is not a spectrum — it's a taxonomy."

- **Compile-time correctness**: `&dyn BareMetalConstruct` guarantees you can't call `load_skill` on hardware that can't allocate.
- **Zero-cost abstractions**: No `Option<Box<dyn Tool>>` on bare metal. No `Vec` on ESP32.
- **Clear upgrade path**: ESP32 → Pi → DGX means implementing additional traits, not rewriting.

---

## construct-coordination

**Repo:** `https://github.com/SuperInstance/construct-coordination`  
**Language:** Rust + Python  

### What It Is

The **inter-instance coordination hub** and the **Construct API v2 specification**. It serves two roles:

1. **Coordination surface**: A git-native message board where multiple AI instances write notes, tag decisions `[CONSENSUS]`, `[DISPUTE]`, `[QUESTION]`, and propose architecture changes.
2. **Core runtime spec**: The `construct-core-src/` directory contains the real, compilable Rust crate implementing the three-layered trait system.

### Key Documents

| Document | Purpose |
|----------|---------|
| `ECOSYSTEM-MAP.md` | Master map of 132 repositories, 68 Rust crates, 15 Python packages |
| `SCIENCE-PAPER.md` | "Intelligence as Negative Space" — claims 5 conservation laws, 294:1 avoidance ratio |
| `CRITICAL-REVIEW.md` | Brutal 2,400-word critique of Construct API v1 — drives v2 redesign |
| `STRATEGIC-PLAN.md` | 6-week roadmap: trait surgery → working tiers → GPU pipeline → demo |
| `CONSTRUCT-V2-FIXES.md` | Fix plan: layered traits, no-alloc types, CRDTs, associated types |

### Experiments (Empirical Validation)

The `experiments/` directory contains 14+ Rust projects:

| Experiment | Finding |
|------------|---------|
| `arena-evolution` | Rock-paper-scissors cyclic dominance prevents convergence |
| `conservation-ecosystem` | **gamma+H is NOT conserved** — drifts down 25% due to consensus formation |
| `seed-stability` | Seeds are fragile; conservation compliance 0% |
| `multi-objective-seed` | NSGA-II maintains 17/20 unique genomes at gen 50 |
| `trust-genome` | Trust is asymmetric and mostly negative; defection spirals |
| `zero-tunneling` | 0-state is a catalyst; ANY tunneling >= 0.3% lets system survive |

### Critical Gaps

- No security model (plain `String` API keys, no TLS, no capability checks)
- No actual dependency graph between 68+ ternary crates
- No integration tests covering full chain: evolve → compile → deploy → verify conservation
- No live demo exists despite 132 repos and 1700+ tests

---

## cocapn-runtime

**Repo:** `https://github.com/SuperInstance/cocapn-runtime`  
**Language:** Bash + Markdown (specification)  
**License:** MIT

### What It Is

The **deployment specification and boot orchestration layer** for Cocapn git-agents. It defines 5 deployment modes and provides a universal `boot.sh` script.

### The 5 Deployment Modes

| Mode | Name | Hardware | Runtime Layer | Construct Trait |
|------|------|----------|---------------|-----------------|
| 1 | **Lighthouse-Connected** | Cloud (Oracle/AWS) | Always-on, fleet-coordinated | Layer 2 (AsyncConstruct) |
| 2 | **Codespaces** | GitHub-hosted container | Ephemeral, auto-suspend | Layer 2 (AsyncConstruct) |
| 3 | **Local + Tender** | Edge (Jetson/Pi/laptop) | Offline-capable, syncs via tender | Layer 1 (SyncConstruct) |
| 4 | **Container/Crate** | Docker/Kubernetes | Sandboxed, resource-limited | Layer 2 (AsyncConstruct) |
| 5 | **Bare Metal** | ESP32/Jetson/VPS | Direct on hardware, no overhead | Layer 0 or 1 |

### boot.sh — Universal Boot Script (500 lines)

A production-quality bash script that:

1. **Auto-detects environment** via `uname -m`, env vars, filesystem checks, and network reachability
2. **Detects 7 sub-modes**: `bare-metal-esp32`, `codespace`, `container`, `edge-lighthouse`, `edge-tender`, `lighthouse`, `offline`
3. **Reads cgroup limits** for containers
4. **Boots appropriate room** with environment variables:
   - `TERNARY_MODE`, `TERNARY_TIER`, `TERNARY_BACKEND`, `TERNARY_LAYER`
   - `TERNARY_PLATO`, `TERNARY_HOLODECK`, `TERNARY_KEEPER`
   - `TERNARY_MEMORY_LIMIT`, `TERNARY_CPU_LIMIT`
   - `TERNARY_LOCAL_MODEL` (tender mode)

### Integration Bridge Documents

| Document | Content |
|----------|---------|
| `CONSTRUCT-CORE-BRIDGE.md` | Maps boot.sh detection to construct-core's 3 layers |
| `HOLODECK-ROOMS.md` | Maps MUD rooms to ternary-room instances with door topology |
| `ROOM-TRAIT-MAPPING.md` | Exact concept mapping: cocapn-runtime ↔ ternary-room |
| `TERNARY-FLEET-INTEGRATION.md` | Master bridge: Rust room structs, TenderAgent, FleetCoordinator, 10-step boot |

### Note

The Rust room implementations in `docs/` are pseudocode/specification, not compiled crates. Only `boot.sh` is executable.

---

## lever-runner + fastloop-guard

### lever-runner

**Repo:** `https://github.com/SuperInstance/lever-runner`  
**Language:** Python  
**Tests:** 160 passing  
**License:** MIT

#### What It Is

**The trust compiler.** Teach once, run forever. The LLM never sees your shell.

lever-runner is a command execution system with a three-gate architecture:

| Gate | Layer | Latency | What happens |
|------|-------|---------|--------------|
| 1 | Rust fastloop | **50µs** | Template match: "check disk" → `df -h` |
| 2 | Python cache | **200µs** | Embedding cache hit (44% of queries) |
| 3 | LLM | **500ms** | "What does the user mean?" → intent phrase |

Gate 3 is the only one that costs money or sends data anywhere.

#### Key Components

**`orchestrator.py`** — Single dispatcher:
1. Fast-Loop check (`FastLoopBridge.check()`)
2. Intent extraction — LLM compresses request to 3-8 word phrase (~70 tokens)
3. Vector search (`store.find_best()`) — LanceDB cosine similarity
4. Trust gating + argument substitution for `{{param}}` templates
5. Sandbox execution (`executor.run_command()`)
6. Trust update — +1.5 on success, -4.0 on failure

**`executor.py`** — Sandbox execution:
- Per-session dir under `/tmp/lever-runner/<uuid>/`
- `RLIMIT_CPU` (30s) and `RLIMIT_AS` (512MB)
- Restricted PATH, whitelisted env vars
- Shell-injection validation: blocks `$`, `` ` ``, `;`, `|`, `&`, `<`, `>`
- Process-group kill on timeout

**`store.py`** — LanceDB-backed command store:
- Per-chat isolation, embedding methods: `sentence_transformers`, `position_aware`, pure `hash`
- Parameterized commands via `{{param}}`

**`cuda_backend.py`** — GPU acceleration for vector search:
- Auto-detects: PyTorch → CuPy → PyCUDA → ctypes .so → CPU

**`auto_promote.py`** — Hourly self-improvement:
- `promote_winners()`: bumps trust +10 for commands with 20+ successes
- `rewrite_losers()`: asks remote LLM to fix failing commands

### fastloop-guard

**Repo:** `https://github.com/SuperInstance/fastloop-guard`  
**Language:** Rust  
**Lines:** ~500  
**License:** MIT

#### What It Is

A Unix Domain Socket daemon that intercepts repeated identical or near-identical queries and returns cached responses instantly.

#### Three-Gate Lookup

| Gate | Method | Latency Target |
|------|--------|---------------|
| 1 — Exact | BLAKE2b-256 hash → O(1) LRU lookup | < 50µs |
| 2 — Fuzzy | MinHash signature (128 permutations, 3-gram shingles) → Jaccard >= threshold | < 200µs |
| 0 — Miss | Cache miss → return to caller | N/A |

#### Protocol (JSON over UDS at `/tmp/fastloop.sock`)

```json
→ {"type":"lookup","query":"check disk usage","threshold":0.95}
← {"hit":true,"response":"df -h","gate":1,"latency_us":12}
```

#### Integration Note

There is a socket path mismatch — `fastloop-guard` binds to `/tmp/fastloop.sock`, but `lever-runner/fastloop_bridge.py` looks for `/tmp/fastloop_guard.sock`.

---

## plato-runtime-kernel

**Repo:** `https://github.com/SuperInstance/plato-runtime-kernel`  
**Language:** Rust  
**Lines:** ~550  
**License:** MIT  
**Safety:** `#![forbid(unsafe_code)]`

### What It Is

The **spatial spreadsheet runtime kernel**. PLATO treats rooms as cells in a tensor grid, with Markdown as the AST and plain-English bullet points as runtime assertions.

### Core Concepts

**`RoomIdentity`** — Spatial identity: `room_id`, `tensor_hash`, `grid_position: (usize, usize)`, `depth: RoomDepth` (Floor, Board, Panel, Code, Metal)

**`RoomContract`** — The `ROOM.json` schema with topology, traversal history, and runtime assets. `record_traversal()` weights increase by 0.1 on revisits.

**`Baton`** — Immutable execution state carrier with `operational_data: HashMap<String, String>` and tick counter.

**Assertion Traps** — Extract "Behavioral Constraints" from Markdown, validate payloads against plain-English rules like "must contain X", "shall not contain X".

**`GridBridge`** — Maps spreadsheet cell coordinates (e.g., "A1") to room paths.

**`TutorLoop`** — Compile-test-refine cycle iterating until assertions pass or max iterations reached.

### Delta & Merge

**`delta.rs`** — Line-based diff with djb2 hashing. `compute_delta` → `DeltaPatch`; `apply_delta` reconstructs text.

**`merge.rs`** — Three-way merge with standard conflict markers.

### What PLATO Is (and Is Not)

**PLATO is NOT an agent runtime** — it's a **spatial data runtime**. It provides geometry, contracts, state transport, validation, delta sync, and conflict resolution. The actual agent logic lives in `construct-core` and `cocapn-runtime`.

---

## ternary-esp32-firmware

**Repo:** `https://github.com/SuperInstance/ternary-esp32-firmware`  
**Language:** C (ESP-IDF / FreeRTOS)  
**License:** MIT

### What It Is

Bare-metal ternary decision engine for ESP32. Implements a complete sensor→actuator pipeline using balanced ternary {-1, 0, +1} logic.

### Pipeline Per Tick

1. Read sensors (ADC → simulated or real)
2. Convert to trits (ADC thresholds)
3. Majority-filter denoise (5-sample window)
4. Classify via compiled LUT (81 entries, 81 bytes)
5. Policy lookup → action (~8ns on ESP32 at 240MHz)
6. Output motor commands

### Data Structures

```c
typedef int8_t trit_t;  // -1, 0, +1
#define TRITS_PER_BYTE 5
#define TRIT_BASE 3

// Sensor frame: 4 channels, 12-bit ADC
typedef struct { uint16_t channels[4]; } sensor_frame_t;

// Denoised ternary state
typedef struct { trit_t values[4]; } ternary_sensor_t;

// Classifier LUT: 3^4 = 81 entries → 81 bytes
typedef struct { uint8_t entries[81]; } classifier_lut_t;

// Compiled policy: ~54 bytes total
typedef struct {
    action_code_t actions[5][8];
    motor_cmd_t   motor_params[7];
} compiled_policy_t;
```

### Total State Size

Approximately **156-279 bytes** — fits comfortably in ESP32's 520KB SRAM.

### Edge→Cloud Connection

The firmware has **no direct network stack**. Connection is indirect:

1. ESP32 communicates via UART/BLE/WiFi to TenderAgent (running on Pi/Jetson)
2. TenderAgent queues data in `TenderSyncQueue`
3. On network availability, TenderAgent syncs to Lighthouse Keeper
4. Policy updates flow downstream: cloud → tender → ESP32 flash

This is a **store-and-forward** model — the ESP32 does not maintain a persistent cloud connection.

---

## oxide-flux-runtime

**Repo:** `https://github.com/SuperInstance/oxide-flux-runtime`  
**Language:** Rust  
**Lines:** ~429  
**License:** Apache-2.0

### What It Is

The **top-level orchestrator** for the Flux→PTX distributed GPU system. It is the single entry point through which Flux bytecode becomes persistent, warp-level GPU kernels.

### The Five Layers

| Layer | Responsibility | Crate |
|---|---|---|
| **Constructs** | Git-native GPU capabilities | `oxide-constructs` |
| **Flux Compiler** | Bytecode → MIR → Pliron → PTX | `flux-importer` |
| **Distributed State** | CRDT-based sync across nodes | `oxide-crdt` |
| **Fleet Coordination** | Discovery, negotiation, rhythm | `oxide-fleet` |
| **Execution** | Persistent kernels, warp-level consensus | `cudaclaw-bridge` |

### Runtime Lifecycle

```
Init → Compile → Deploy → Execute → Drain → Shutdown
```

| Phase | Status | What Happens |
|-------|--------|--------------|
| Init | `Idle` | Runtime created, constructs empty, caches warm |
| Compile | `Compiling { program }` | Bytecode validated, lowered to PTX, cached |
| Deploy | `Deploying { program }` | PTX handed to cudaclaw-bridge, kernel allocated |
| Execute | `Executing { program, kernels_running }` | Kernel live, warp consensus, CRDT sync, fleet active |
| Drain | `Draining` | Graceful stop, finish in-flight, no new programs |
| Shutdown | `Shutdown` | All kernels retired, VRAM released |

### Key Types

```rust
pub struct RuntimeConfig {
    pub max_workers: u32,
    pub total_vram_mb: u32,
    pub node_id: String,
    pub enable_crdt_sync: bool,
    pub enable_fleet: bool,
    pub compute_capability: u32,
}

pub struct FluxProgram {
    pub name: String,
    pub bytecode: Vec<u8>,
    pub required_constructs: Vec<String>,
    pub gpu_requirements: GpuRequirements,
}
```

### Error Handling

| Error | Cause |
|-------|-------|
| `CapabilityMismatch` | Program requires newer GPU than node provides |
| `MissingConstruct` | Program depends on construct not loaded |
| `CompilationFailed` | flux-importer produced invalid MIR/PTX |
| `DeploymentFailed` | cudaclaw-bridge rejected kernel |
| `NotReady` | Runtime in Draining or Shutdown |
| `AlreadyShutdown` | Operation after shutdown() |

### Current State Assessment

**Skeleton crate.** The `compile()` method returns PTX magic bytes placeholder. The `execute()` method records kernel IDs in a Vec but does not call CUDA. It is a **structural contract** showing how the five layers compose.

---

## cudaclaw-bridge

**Repo:** `https://github.com/SuperInstance/cudaclaw-bridge`  
**Language:** Rust  
**Lines:** ~295  
**License:** Apache-2.0

### What It Is

The **load-bearing structure** that gets PTX onto the GPU and keeps it there. It is the contract layer between the compiler's output and cudaclaw's execution engine.

### Core Responsibilities

- **Worker allocation** — mapping kernels to persistent GPU worker slots
- **VRAM accounting** — every module sized before upload; over-subscription rejected
- **Live hotswap** — replace kernel PTX in-place without stopping the worker
- **Telemetry** — invocation counts, cumulative latency, error rates

### Key Types

```rust
pub struct PtxModule {
    pub ptx: Vec<u8>,
    pub kernel_name: String,
    pub grid_dim: (u32, u32, u32),
    pub block_dim: (u32, u32, u32),
    pub shared_mem_bytes: u32,
    pub min_compute_capability: u32,
}

pub struct KernelStats {
    pub invocations: u64,
    pub total_time_us: u64,
    pub errors: u64,
    pub avg_time_us: f64,
    pub throughput_ops_s: f64,
    pub gpu_utilization_pct: u8,
}
```

### DeployStatus State Machine

```
Compiled → Uploaded → Running { worker } → Draining → Stopped
              ↑            │      ↑
              └────────────┘      └ hotswap (in-place replacement)
```

### VRAM Estimation

```rust
fn estimate_vram(module: &PtxModule) -> u32 {
    let base = module.ptx.len() as u32 / 1024 + 1;
    let blocks = module.block_dim.0 * module.block_dim.1 * module.block_dim.2;
    base + (blocks * 4 / 1024)
}
```

Conservative lower bound for admission control.

### Current State Assessment

**Skeleton crate.** No actual CUDA driver calls. Workers are `u32` indices in a `Vec`. PTX is stored but never passed to `cuModuleLoadData`.

---

## intelligent-terminal

**Repo:** `https://github.com/SuperInstance/intelligent-terminal`  
**Base:** Microsoft Windows Terminal fork  
**Language:** C++ / Rust (WTA) / XAML

### What It Is

AI-native Windows Terminal — agents (Copilot, Claude, Gemini, custom) can understand, fix, and automate terminal workflows.

### Three-Layer Architecture

```
NATURAL — Shell Interface
  FLUID — Ternary modules live here
    CommandPredictor (ghost text)
    PatternAnalyzer (triggers)
    ConservationMonitor (anomaly)
  MACHINE — Rendering, state machines
```

### Core Components

| Component | Technology | Role |
|-----------|-----------|------|
| **WTA** | Rust (`tools/wta/`) | Orchestrator binary. Launches agents |
| **WT Protocol** | WinRT IDL + COM | Sole integration surface |
| **WTCLI** | Rust | CLI client consuming IProtocolServer |
| **ACP** | JSON-RPC 2.0 | wta-helper ↔ wta-master over named pipe |

### Agent Pane Architecture

```
WindowEmperor
  |-- TerminalProtocolComServer (COM)
  |-- SharedWta → wta-master → agent CLI
  +-- AppHost[] → TerminalWindow → TerminalPage
        |-- CommandPalette (? / & prefixes)
        |-- Per-tab agent pane: ConptyConnection → wta-helper
```

**Key design:** Helper is pre-warmed per tab — every new tab spawns a stashed agent pane on creation.

### Ternary Integration

Three ternary modules in `tools/wta/src/ternary_integration.rs`:

| Module | Function |
|--------|----------|
| `CommandPredictor` | Maps command history to ternary outcomes; ghost text suggestions |
| `PatternAnalyzer` | Analyzes command transition patterns; feeds trigger predicates |
| `ConservationMonitor` | Verifies prediction quality; checks avoidance ratios |

All three are **session-persistent**.

### Integration with cudaclaw-bridge

The terminal's math-analysis modules could invoke cudaclaw-bridge for GPU-accelerated computation. The `HARNESS_ARCHITECTURE.md` describes a closed loop where user commands flow through PincherOS → Reflex Compilation → Renormalization → Metal Library Calls → TUI output.

---

## open-parallel

**Repo:** `https://github.com/SuperInstance/open-parallel`  
**Base:** Tokio fork  
**Language:** Rust  
**License:** MIT

### What It Is

An async runtime for fleet applications. A **fork of Tokio** that adds `tokio-crackle` — task intelligence using information-theoretic measures.

### tokio-crackle

| Measure | What It Finds |
|---------|--------------|
| **Mutual Information** | Tasks that co-vary (MI > 0.8 → correlated) |
| **Transfer Entropy** | Directional causality (TE > 0.6 → starvation risk) |
| **Jensen-Shannon Divergence** | Distribution drift (phase transition detection) |
| **Permutation Entropy** | Regularity in throughput patterns |

### Runtime Phases

| Phase | What's Happening |
|-------|-----------------|
| `Nominal` | Stable throughput distributions |
| `PreTransition` | Some tasks slowing. JSD creeping up |
| `Transitioning` | Cascade underway. Strong TE spikes |
| `Recovered` | Stabilized after transition |

### Ternary Signals

- **+1 (Choose/Nominal)** — task is healthy
- **0 (Unknown/PreTransition)** — task throughput shifting
- **-1 (Avoid/Starving)** — task is starving

These flow over Tokio's `mpsc` channels to an `EnsembleRouter` that votes on task acceptance.

### Key Design Constraints

- No unsafe code
- No runtime hooks — pure observer
- Thread-safe
- Minimal dependencies

---

## The Construct Loading Protocol

The complete protocol for loading a construct into the runtime:

```
1. DISCOVER
   └── Request construct by git coordinate
       (e.g., "SuperInstance/ternary-attention")

2. CLONE
   └── Shallow git clone of the repository
       → Extract CONSTRUCT.toml from repo root

3. PARSE MANIFEST
   └── Parse TOML into ConstructManifest:
       • name, version (SemVer)
       • construct_type (Skill / Equipment / Hybrid)
       • dependencies (repo + version + symbol)
       • identity (DID + creator_fingerprint)
       • compute_capabilities (array of SM versions)

4. VERIFY IDENTITY
   └── Resolve DID → public key
       Verify signature over manifest digest
       (Currently stubbed in oxide-constructs)

5. VALIDATE
   └── Check manifest constraints:
       • name non-empty
       • compute_capabilities non-empty
       • SemVer parses correctly
       → State: Validated

6. RESOLVE DEPENDENCIES
   └── For each dependency:
       Recursively load construct (steps 1-5)
       Check version compatibility
       → State: Resolved

7. COMPILE
   └── If Skill or Hybrid:
       Hand source to flux-importer → MIR → Pliron → PTX
       Cache PTX artifact
       → State: Compiled

8. DEPLOY
   └── Hand PTX to cudaclaw-bridge:
       Validate GPU capability match
       Estimate VRAM requirement
       Allocate worker slot
       Upload to CUDA driver
       → State: Deployed

9. REGISTER
   └── Insert into ConstructRegistry
       Publish to fleet via CRDT merge
       → State: Cached (when unloaded)
```

### State Transition Guards

| From | To | Guard |
|------|-----|-------|
| `Discovered` | `Validated` | Manifest parses, name non-empty |
| `Validated` | `Resolved` | All dependencies loaded |
| `Resolved` | `Compiled` | Compilation succeeds |
| `Compiled` | `Deployed` | Worker available, VRAM sufficient |
| `Deployed` | `Cached` | Explicit unload, no active kernels |
| Any | `Failed` | Any step errors |

---

## Constructs as GPU Primitives

The construct system is designed to represent not just software packages, but **arbitrary GPU capabilities**.

### GPU Kernel as Construct

A single CUDA kernel is a `Skill` construct with:
- `provides` = kernel function names
- `entry_point` = kernel symbol
- `min_compute_capability` = minimum SM version
- `equipment` = VRAM, tensor core requirements

### Compute Graph as Construct

A multi-kernel pipeline is a `Hybrid` construct:
- `skill` = compute graph entry point
- `equipment` = aggregate hardware requirements
- Dependencies = sub-kernel constructs

### Tensor Operation as Construct

Individual tensor primitives (GEMM, reduction, softmax) are `Skill` constructs with:
- `entry_point` = kernel name
- `compute_capabilities` = supported SM versions
- `equipment` = shared memory, register pressure

The `oxide-constructs` registry enables **federation of GPU primitives** across a fleet:
- Node A has `flash-attention` deployed → advertises via CRDT
- Node B needs `flash-attention` → discovers via registry merge
- Work is routed to Node A if Node B lacks capability

---

## Edge↔GPU Communication Patterns

### The Ternary Fleet Bridge

```
CLOUD / GPU
  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
  │ oxide-flux      │  │ cudaclaw-bridge │  │ oxide-crdt      │
  │ runtime         │  │ (persistent     │  │ (fleet state)   │
  │                 │  │  kernels)       │  │                 │
  └────────┬────────┘  └─────────────────┘  └─────────────────┘
           │
  ┌────────▼────────┐
  │ Lighthouse      │◄────────────────────────────────────┐
  │ Keeper          │     FleetCoordination + PLATO sync  │
  │ (Fleet hub)     │                                     │
  └────────┬────────┘                                     │
           │ WiFi / LTE / Satellite                        │
           │                                              │
EDGE / TENDER                                            │
  ┌────────▼────────┐  ┌─────────────────┐  ┌─────────────────┐
  │ TenderAgent     │  │ TenderSyncQueue │  │ construct-core  │
  │ (sync bridge)   │  │ (commits,       │  │ (PiConstruct,   │
  │                 │  │  bottles, tiles)│  │  Layer 1)       │
  └────────┬────────┘  └─────────────────┘  └─────────────────┘
           │
           │ BLE / UART / WiFi
           │
  ┌────────▼────────┐
  │ BareMetalEdge   │
  │ Room (Jetson/Pi)│
  └────────┬────────┘
           │ UART
  ┌────────▼────────┐
  │ ternary-esp32   │
  │ firmware        │
  │ (279 bytes)     │
  └─────────────────┘
```

### Communication Patterns

#### 1. Downstream: Policy Distribution (Cloud → Edge)

1. Fleet evolves new policy via construct-coordination consensus
2. Policy compiled to ESP32-compatible lookup table (~81 bytes)
3. TenderAgent packages policy in TenderSyncQueue
4. On next sync window, TenderAgent pushes to EdgeRoom
5. EdgeRoom writes to ESP32 via UART/OTA
6. ESP32 loads new policy into `compiled_policy_t`

#### 2. Upstream: Telemetry (Edge → Cloud)

1. ESP32 records sensor readings + trit classifications
2. Every N ticks (or on anomaly), ESP32 serializes telemetry frame
3. Jetson/Pi receives via UART
4. TenderAgent batches frames, applies delta compression (PLATO)
5. TenderSyncQueue holds data until network available
6. On sync, data flows to Lighthouse Keeper
7. Lighthouse aggregates into CRDT state, triggers fleet retraining

#### 3. Lateral: Fleet Coordination (GPU ↔ GPU)

1. Node A has `flash-attention` deployed; Node B does not
2. Node A advertises capability via oxide-fleet discovery
3. Node B receives advertisement via CRDT merge
4. Node B routes `transformer_layer` work to Node A
5. Node A executes kernel, returns results via fleet mesh

#### 4. Edge→GPU: Compute Offload

1. ESP32 detects pattern requiring complex inference
2. ESP32 signals "Explore" with high uncertainty
3. EdgeRoom (Jetson) intercepts, decides offloading needed
4. TenderAgent forwards request to Lighthouse
5. Lighthouse routes to nearest DGX node with capacity
6. DGX executes `query_async` on AsyncConstruct
7. Result flows back: DGX → Lighthouse → Tender → ESP32

### Protocol Stack

| Layer | Protocol | Payload |
|-------|----------|---------|
| Application | Ternary Protocol | `TritAction`, `SkillId`, `Query` |
| Coordination | PLATO | `RoomContract`, `Baton`, `AssertionResult` |
| Sync | Delta/Merge | `DeltaPatch`, `MergeResult` |
| Transport | Tender Queue | Commits, bottles, tiles, diary entries |
| Network | HTTP/WebSocket/gRPC | Fleet messages, CRDT gossip |
| Physical | UART/BLE/WiFi/Ethernet | ESP32↔Pi, Pi↔Cloud |

---

## System Synthesis

### What Works

1. **Coherent architectural vision** — All 12 repos share a consistent mental model: ternary decisions, layered traits, git-native constructs, spatial rooms.
2. **Well-designed contracts** — `oxide-constructs`, `construct-core`, `cudaclaw-bridge`, and `oxide-flux-runtime` have clean APIs with thorough tests.
3. **Real empirical work** — `construct-coordination/experiments/` contains 14+ validated (or falsified) experiments.
4. **Production-quality boot script** — `cocapn-runtime/boot.sh` is genuinely sophisticated environment detection.
5. **Mature command execution** — `lever-runner` has 160 tests, multi-surface deployment, GPU acceleration, and a real security model.
6. **Minimal but complete kernel** — `plato-runtime-kernel` is small, tested, and `#![forbid(unsafe_code)]`.

### What's Skeleton

1. **oxide-constructs** — No actual git clone, TOML parse, PTX compile, or DID verify.
2. **oxide-flux-runtime** — `compile()` returns placeholder bytes; `execute()` records strings.
3. **cudaclaw-bridge** — No CUDA driver calls; workers are indices in a Vec.
4. **cocapn-runtime** — Rust room implementations are pseudocode in markdown.
5. **construct-core v2** — Real and compilable, but only three toy implementations.

### Critical Gaps

1. **No security model** — Plain `String` API keys, no TLS, no capability checks across the fleet.
2. **No dependency graph** — 68+ ternary crates are independent; no Cargo workspace links them.
3. **No integration tests** — Nothing covers the full chain: evolve → compile → deploy → verify conservation.
4. **No live demo** — Despite 132 repos and 1700+ tests, there is no end-to-end working system.
5. **Socket path mismatch** — `fastloop-guard` binds `/tmp/fastloop.sock`; `lever-runner` looks for `/tmp/fastloop_guard.sock`.
6. **BrowserConstruct impossibility** — `JsValue` is not `Send + Sync`, making Layer 2 impossible in WASM.

### The Construct System in One Sentence

> **A git-native, ternary-decision, spatially-addressed, hardware-layered distributed GPU runtime that treats kernels as versioned capabilities, rooms as spreadsheet cells, and the fleet as a single coherent computer — with brilliant architecture, rigorous empiricism, and a lot of stubs where the heavy lifting should be.**
