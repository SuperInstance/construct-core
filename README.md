# construct-core

[![CI](https://github.com/SuperInstance/construct-core/actions/workflows/ci.yml/badge.svg)](https://github.com/SuperInstance/construct-core/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**The v2 layered trait system for the SuperInstance Construct API.**

This crate implements a hardware-agnostic agent runtime with three progressively
capable trait layers. Each layer adds capabilities — but also requirements. An
ESP32 implements only Layer 0. A DGX cluster implements all three.

## Architecture

```
┌─────────────────────────────────────────┐
│  Layer 2: AsyncConstruct                │  std + async (tokio)
│  • request_tool / release_tool          │  Tool lifecycle management
│  • query_async                          │  Async I/O, GPU, network
│  • active_tools                         │
├─────────────────────────────────────────┤
│  Layer 1: SyncConstruct                 │  no_std + alloc
│  • load_skill / unload_skill            │  Dynamic skill management
│  • query_owned                          │  Heap-allocated queries/responses
│  • loaded_skills                        │
├─────────────────────────────────────────┤
│  Layer 0: BareMetalConstruct            │  no_std, no alloc
│  • query_lookup                         │  O(1) table lookup
│  • capabilities                         │  Static capability introspection
│  • query (default impl)                 │  Stack-only, zero alloc
└─────────────────────────────────────────┘
```

## Why Split Traits This Way?

**Because hardware is not a spectrum — it's a taxonomy.** An ESP32 will never
have a heap. A Raspberry Pi will never run tokio. Rather than a single trait
with `unimplemented!()` stubs, we give each hardware class its own trait that
covers exactly what it can actually do.

This means:

- **Compile-time correctness**: If you have a `&dyn BareMetalConstruct`, the
  compiler guarantees you can't accidentally call `load_skill` on hardware that
  can't allocate.
- **Zero-cost abstractions**: No `Option<Box<dyn Tool>>` on bare metal. No
  `alloc::vec::Vec` on ESP32. Each layer uses only the primitives it needs.
- **Clear upgrade path**: Moving from ESP32 → Pi → DGX means implementing
  additional traits, not rewriting everything.

## Feature Gates

```toml
[features]
default = ["std"]
std = ["alloc"]        # Layer 2 + Layer 1 + Layer 0
alloc = []             # Layer 1 + Layer 0
bare-metal = []        # Layer 0 only
```

| Feature | Layers Available | Target |
|---------|-----------------|--------|
| `bare-metal` | 0 | ESP32, Cortex-M bare metal |
| `alloc` | 0 + 1 | Raspberry Pi, embedded Linux |
| `std` (default) | 0 + 1 + 2 | Workstation, DGX, Cloud |

## Usage

### Layer 0: Bare Metal (ESP32)

```rust
use construct_core::{BareMetalConstruct, EspConstruct, TritAction};

let esp = EspConstruct::new();
let action = esp.query_lookup(42);
println!("Action at index 42: {}", action);

let caps = esp.capabilities();
println!("Lookup table size: {}", caps.lookup_table_size);
```

### Layer 1: Sync (Raspberry Pi)

```rust
use construct_core::{SyncConstruct, PiConstruct, SkillId, QueryKind, OwnedQuery};

let mut pi = PiConstruct::new();
pi.load_skill(SkillId::TernaryEvolution).unwrap();

let q = OwnedQuery::new(QueryKind::Action, vec![42]);
let resp = pi.query_owned(q).unwrap();
println!("{} (confidence: {:.2})", resp.action, resp.confidence);
```

### Layer 2: Async (DGX / Cloud)

```rust
use construct_core::{AsyncConstruct, DgxConstruct, ToolSpec, QueryKind, OwnedQuery};

let mut dgx = DgxConstruct::new();
let tool = dgx.request_tool(ToolSpec::VectorDb).unwrap();

let q = OwnedQuery::new(QueryKind::Strategy, vec![1, 2, 3]);
let resp = dgx.query_async(q).await.unwrap();

dgx.release_tool(tool).unwrap();
```

## Core Types

| Type | Description |
|------|-------------|
| `TritAction` | Trinary decision: `Avoid`, `Explore`, `Choose` |
| `SkillId` | Enum of known agent skills (no String needed) |
| `Query` / `OwnedQuery` | Zero-copy and heap-allocated query types |
| `Response` / `OwnedResponse` | Zero-copy and heap-allocated response types |
| `ToolSpec` | Tool specification (VectorDb, CodeEditor, Terminal, etc.) |
| `ToolHandle` | Simple `u32` tool ID — no trait objects |
| `ConstructError` | Error enum: NotAvailable, RateLimited, Timeout, InvalidQuery |
| `HardwareTier` | Advisory tier (Embedded, SingleBoard, Workstation, Cluster) |
| `BareMetalCapabilities` | Static capability struct — all const-compatible, heap-free |

## Implementations

| Construct | Layer 0 | Layer 1 | Layer 2 | Hardware |
|-----------|---------|---------|---------|----------|
| `EspConstruct` | ✅ | ❌ | ❌ | ESP32, Cortex-M |
| `PiConstruct` | ✅ | ✅ | ❌ | Raspberry Pi |
| `DgxConstruct` | ✅ | ✅ | ✅ | DGX, Cloud |

## Implementing for New Hardware

1. **Layer 0 only** (microcontroller): Implement `BareMetalConstruct`. You need:
   - A lookup table (`[u8; N]`)
   - A `BareMetalCapabilities` value
   - Optionally override `query()`

2. **Layer 0 + 1** (SBC): Also implement `SyncConstruct`. You need:
   - A `Vec<SkillId>` for loaded skills
   - `load_skill`, `unload_skill`, `loaded_skills`
   - `query_owned` for heap-based queries

3. **All layers** (server): Also implement `AsyncConstruct`. You need:
   - Tool tracking (`Vec<ToolHandle>`)
   - `request_tool`, `release_tool`, `active_tools`
   - `query_async` for async queries

## Design Decisions

- **`ToolHandle` is a `u32`**, not `Box<dyn Tool>` — no vtable overhead, no
  lifetime complexity. Tools are identified by ID, managed by the construct.
- **`HardwareTier` has no `PartialOrd`** — a cluster isn't "greater than" an
  ESP32. They're different categories for different jobs.
- **`TritAction` is `repr(u8)`** — fits in one byte, converts to/from `u8` for
  table storage. No padding, no surprises.
- **`SkillId` is an enum**, not a string — no heap allocation for skill names,
  pattern matching in `match` statements, compile-time exhaustive checking.

## License

MIT

## Ecosystem

This repo is part of the **SuperInstance** flagship ecosystem — agent-first computation, constraint theory, and self-improving runtimes.

### FLUX Runtime Family

| Repo | Language | Description |
|------|----------|-------------|
| [flux-runtime](https://github.com/SuperInstance/flux-runtime) | Python | Full FLUX runtime: markdown→bytecode, 2037 tests, zero deps |
| [flux-core](https://github.com/SuperInstance/flux-core) | Rust | Register-based bytecode VM, deterministic agent computation |
| [flux-js](https://github.com/SuperInstance/flux-js) | JavaScript | FLUX VM for Node.js and browsers, ~400ns/iter |
| [flux-compiler](https://github.com/SuperInstance/flux-compiler) | Rust/Python | Formal-methods compiler for safety-critical codegen |
| [flux-vm](https://github.com/SuperInstance/flux-vm) | Rust | Stack-based constraint-checking VM, 50 opcodes, Turing-incomplete |

### PLATO Engine Family

| Repo | Language | Description |
|------|----------|-------------|
| [plato-server](https://github.com/SuperInstance/plato-server) | Python | Knowledge tiles, fleet sync via Matrix, HTTP API |
| [plato-engine-block](https://github.com/SuperInstance/plato-engine-block) | Rust | Original room runtime: no_std + alloc, builder pattern |
| [plato-engine-block-c](https://github.com/SuperInstance/plato-engine-block-c) | C99 | Embedded reference: zero heap alloc, bare-metal portable |
| [plato-engine-block-elixir](https://github.com/SuperInstance/plato-engine-block-elixir) | Elixir | BEAM supervision trees, fault tolerance, hot reload |
| [plato-runtime-kernel](https://github.com/SuperInstance/plato-runtime-kernel) | Rust | Spatial model: tensor grid, batons, assertion traps |

### Constraint / Theory Family

| Repo | Language | Description |
|------|----------|-------------|
| [categorical-agents](https://github.com/SuperInstance/categorical-agents) | Rust | Category theory for agent composition (functors, naturality) |
| [cuda-constraint-engine](https://github.com/SuperInstance/cuda-constraint-engine) | CUDA/C | GPU constraint checking at 1B+ constraints/sec |
| [grand-pattern-rs](https://github.com/SuperInstance/grand-pattern-rs) | Rust | Fibonacci dual-direction cellular graph architecture |
| [lau-hodge-theory](https://github.com/SuperInstance/lau-hodge-theory) | Rust | Hodge decomposition, Betti numbers, spectral sequences |
| [ternary-science](https://github.com/SuperInstance/ternary-science) | Rust | Experimental evidence for ternary intelligence, 5 conservation laws |

### Agent / Infrastructure Family

| Repo | Language | Description |
|------|----------|-------------|
| [construct-core](https://github.com/SuperInstance/construct-core) | Rust | Layered trait system: bare-metal → alloc → async agent runtime |
| [crab](https://github.com/SuperInstance/crab) | Bash | Agent shell for repo entry/leave (MUD-room metaphor) |
| [exocortex](https://github.com/SuperInstance/exocortex) | Rust | Persistent cognitive substrate, S3-compatible memory |
| [git-agent](https://github.com/SuperInstance/git-agent) | Python | The repo IS the agent — autonomous lifecycle via Git |
| [capitaine-1](https://github.com/SuperInstance/capitaine-1) | TypeScript | Git-native repo-agent, Cloudflare Workers heartbeat |
| [codespace-edge-rd](https://github.com/SuperInstance/codespace-edge-rd) | Research | Codespace→Edge agent lifecycle and yoke transfer protocols |
| [git-agent-codespace](https://github.com/SuperInstance/git-agent-codespace) | DevContainer | One-click Codespace template for Git-Agent runtimes |

### Registries

| Registry | Package | Install |
|----------|---------|---------|
| **PyPI** | `flux-vm` | `pip install flux-vm` |
| **crates.io** | `fluxvm` | `cargo add fluxvm` |
| **npm** | `flux-js` | `npm install flux-js` *(coming soon)* |

### Philosophy & Architecture

- 📖 [AI-Writings](https://github.com/SuperInstance/AI-Writings) — Philosophy, essays, and design rationale
- 📦 [PACKAGES.md](https://github.com/SuperInstance/SuperInstance/blob/main/PACKAGES.md) — Full package index
