# Changelog

All notable changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [1.0.0] — 2025-09-12

### 🚨 Breaking Changes

- **Experiment → Session Model Overhaul**
  - All code, data structures, database tables, and API endpoints now use "session" terminology in place of "experiment".
  - Database schema changed: `experiment_info` → `session_info`, `experiment_id` → `session_id`, etc.
  - **Data files, config files, and existing ClickHouse databases must be migrated manually**.

- **CLI and Subcommands Refactored**
  - The CLI now uses subcommands: `run`, `view`, and `serve`.
  - Command-line argument names, config expectations, and invocation patterns have changed.
  - The TUI viewer is now invoked via `rex view` instead of the separate `rex-viewer` binary.

- **Python Extension Module Support Removed**
  - All `pyo3`-based extension module code and direct Python integration have been removed.
  - The focus is now on the Rust CLI and Rust-native workflow.

- **Results Handling**
  - Results are now a first-class entity, with a ClickHouse table (`results_store`) and new serialization logic.
  - The protocol for submitting results and session finalization has changed.

- **Measurement Data Structure**
  - Device measurement data now includes explicit units and timestamps.
  - All measurement/channel data is now keyed and stored with units and time series.

- **API/Serialization Changes**
  - Structs and serialization for session, device, and measurement data have changed.
  - Session metadata and validation are enforced on ingestion.

---

### ✨ New Features

- **Session Metadata & Validation**
  - Sessions now support custom metadata fields, which can be validated based on your config.
  - Configurable required metadata keys via the config file.

- **Results Storage and Querying**
  - Results can be sent, stored, and queried through the new `results_store` ClickHouse table.

- **REST API Server Mode**
  - New `rex serve` subcommand runs a web server using Axum, exposing REST endpoints for session control and data access.
  - Endpoints for running, pausing, resuming, and killing sessions, as well as fetching state.

- **Improved TUI Viewer**
  - TUI now supports reconnecting, connection loss warnings, and more robust device/stream selection.
  - Chart state clears on disconnection.

- **Nix Flake, Docker Compose, and Justfile Support**
  - Added development tooling for Nix, Just, and Docker Compose.
  - Easy devshells, reproducible environments, and updated Docker images (ClickHouse + Grafana).

- **Configurable Ports and Paths**
  - Session/server ports and output paths are now more flexible and environment-variable driven.

- **Enhanced Logging and Error Reporting**
  - Improved log formatting, error propagation, and shutdown messaging across all components.

---

### 🛠️ Bug Fixes & Improvements

- Improved error handling and validation throughout.
- More robust device update and session validation logic.
- Better type safety for session metadata and results.
- Refactored and cleaned up codebase, removed dead code and legacy APIs.
- Fixed overflow error in [#3](https://github.com/JaminMartin/rex/issues/3)
---

### 🔒 Security & License

- **License Change:** Project is now licensed under GNU AGPL v3 (see LICENSE file).

---

### ⚙️ Internal Refactors

- Restructured repo: new server module, more modular CLI and core logic.
- Rust workspace dependencies unified and upgraded.
- Test suite updated for new session and measurement models.

---

### 📚 Documentation & Dev Experience

- Example configs, Nix/Flake, Docker Compose, and Justfile included.
- Updated pyproject.toml and maturin scripts for CLI-centric builds.
- More example projects and usage notes in README.md (see Roadmap below).

---

### 🚧 Deprecations/Removals

- Removed all legacy experiment loader logic and related tests.
- Dropped legacy experiment/device API endpoints and code.
- Old Python bindings and extension features fully removed.

---

## Migration Notes

- **Database Migration Required:**
  Existing experiment-based tables/data must be manually migrated to the new session-based schema.
- **Config and Script Updates:**
  Update all scripts, configs, and dashboards to use new session field names.
- **CLI Usage:**
  All users must update CLI invocation patterns and arguments.
- **Python/Extension Users:**
  No longer supports Python extension modules; use the Rust CLI instead.

---

## Recommendations

- See the README.md for updated usage instructions and migration tips.
- Refer to the Docker Compose and Nix files for modern development and deployment.
- If you have custom scripts or dashboards, update them for the new session/results-based model.
- For questions or migration help, open an issue or discussion on GitHub.

---
