# Patina

Last verified: 2026-06-01

## About
NES emulator written in Rust. Targets mapper 0 games (SMB, classic black-box titles) with expanding mapper support.

## Tech Stack
- Language: Rust (2021 edition)
- Windowing/rendering: `winit` + `pixels`
- Audio: `rodio`
- Testing: `cargo test` + `cargo-llvm-cov` for coverage

## Commands
- `cargo build --release` — production build
- `cargo test` — run all tests
- `cargo llvm-cov` — generate coverage report (check % before committing)
- `cargo run -- <rom.nes>` — run a ROM

## Project Structure
- `src/cpu/` — 6502 CPU, addressing modes, memory bus
- `src/ppu/` — Picture Processing Unit, rendering, sprites
- `src/apu/` — Audio Processing Unit, all channels
- `src/mapper/` — Cartridge mappers (NROM, MMC1, UxROM, AxROM)
- `src/simulator/` — Scheduler, program state, main emulation loop
- `src/rom.rs` — ROM loading and iNES header parsing
- `src/window.rs` — Windowing and rendering
- `src/main.rs` — Entry point and CLI (clap)

## Commit Rules

**Every commit must:**
1. Include test code for any new or changed logic
2. Pass `cargo test` — run it before committing
3. Not decrease overall coverage — run `cargo llvm-cov` and verify % is equal or higher than before your change

These are hard rules, not guidelines.

## Commit Message Format

Issue-linked work:
```
Issue #N: <short description>
```

Non-issue work (CI fixes, refactors, tooling):
```
<short description>
```

The description should complete the sentence "This commit will…" and be written in imperative mood. Examples from this repo:
- `Issue #5: Load ROM at runtime via file dialog, Ctrl+Q to exit`
- `Issue #26: add PPU unit and integration tests`
- `Fix APU panicking in headless environments (e.g. CI)`

## Conventions
- Tests live in a `tests/` subdirectory within each module (e.g. `src/ppu/tests/`)
- No mocks for the memory bus — integration-style tests that exercise real wiring
- Mappers implement the `Mapper` trait (`src/mapper/mapper.rs`)
