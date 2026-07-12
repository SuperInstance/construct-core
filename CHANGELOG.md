# Changelog

All notable changes to this project will be documented in this file.

## [0.1.2] - 2025-01-01

### Added
- Three-layered trait system: BareMetalConstruct, SyncConstruct, AsyncConstruct
- Hardware implementations: EspConstruct (ESP32), PiConstruct (Raspberry Pi), DgxConstruct (DGX)
- Core types: TritAction, SkillId, ToolHandle, HardwareTier, ConstructError
- Feature gates: default (std), alloc, bare-metal
- `no_std` compatibility for Layer 0
