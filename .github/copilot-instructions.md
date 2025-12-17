# CortexOS Copilot Instructions

This repo is currently a **blueprint-only** project (no implementation yet). Source of truth is [README.md](README.md).

## Non‑negotiable priorities
- **Run anywhere**: core must compile/run on native + **WASM/WASI**.
- **Spread widely**: Grid should enable node discovery + cooperation via **open, small, versioned protocols**.

## Architecture (what to preserve when adding code)
- **Core stays OS‑agnostic**: implement `core` as traits + event schemas; put platform specifics in adapters.
- **Event‑log first**: model perceptions/actions/network messages as timestamped events; build derived indexes/graphs later.
- **Backpressure by default**: every subscription/queue defines load behavior (drop/coalesce/sample/persist).
- **Capability security**: agents only act through explicit capability tokens (no ambient authority).

## Implementation guidance (first code to write)
- Start from the README roadmap: **Milestone 0.1 = portable runtime + event model**, not hardware.
- Target portability baseline early: keep a **WASI build** green (use the Rust WASI target).
- Do not hard-code the exact WASI target/tooling in docs yet; choose it when `Cargo.toml` is introduced.
- Prefer minimal deps and stable Rust; avoid OS‑specific APIs in `core`.

## Grid / protocols
- Treat Grid messages as a **wire protocol**: binary, versioned, forwards‑compatible.
- Prefer content addressing (hashes) for artifacts and event chunks to simplify syncing.

## Repo conventions (until code exists)
- Do not invent large subsystems not described in the README.
- When proposing new modules, keep them aligned with the proposed dirs in README: `core/`, `grid/`, `signal/`, `agent/`, `sensor/`, `lang/`.

## Low-context / low-tokens workflow
- Prefer **small diffs** over big rewrites; touch the minimum number of files.
- When unsure, **ask 1 question max**; otherwise pick the simplest assumption consistent with the README.
- Avoid long explanations; reference the README sections (e.g., “MVP Interfaces (v0)”).
- When adding specs, keep them **wire-level and versioned** (fields + constraints), not prose.
- Do not introduce heavy deps early; keep interfaces stable and portable.
