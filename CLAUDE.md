# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Scrim is a cross-platform, keyboard-driven desktop navigation system inspired by Homerow and Vimium. It uses accessibility APIs to introspect UI elements and displays hint labels (A, S, D, F...) that users can type to click elements without using a mouse.

The project uses Rust + Tauri v2 architecture with platform-specific accessibility implementations.

## Build Commands

```bash
# Build entire workspace
cargo build

# Build and run the Tauri app (GUI)
cd src-tauri && cargo tauri dev

# Build the CLI tool
cargo build -p scrim-cli

# Run CLI commands
cargo run -p scrim-cli -- permissions    # Check accessibility permissions
cargo run -p scrim-cli -- dump           # Dump UI tree (text format)
cargo run -p scrim-cli -- dump --format json --all  # JSON with all elements

# Run tests
cargo test                               # All tests
cargo test -p scrim-core                 # Core crate only

# Frontend (ui/)
cd ui && npm install                     # Install dependencies
cd ui && npm run dev                     # Dev server
cd ui && npm run build                   # Production build
```

## Architecture

### Crate Structure

```
scrim/
├── crates/
│   ├── scrim-core/       # Platform-agnostic types and traits
│   ├── scrim-macos/      # macOS Accessibility API implementation
│   └── scrim-cli/        # CLI inspection tool
├── src-tauri/            # Tauri application (GUI overlay)
└── ui/                   # TypeScript/Vite frontend for overlay
```

### Core Abstractions (scrim-core)

- `AccessibilityClient` trait: Platform-agnostic interface for accessibility APIs
- `UIElement`: Represents an interactive UI element with id, role, label, rect, actions
- `Rect`: Screen coordinates (top-left origin, logical pixels across all platforms)
- `ElementFilter`: Filters for traversing UI trees (All, Roles, HasActions)
- `HintGenerator`: Generates labels (A, S, D... AA, AS, AD...) using home row keys

### Platform Implementation Pattern

Each platform crate implements `AccessibilityClient`:
- `scrim-macos`: Uses `accessibility`/`accessibility-sys` crates for macOS AX API
- Future: `scrim-windows` (UI Automation), `scrim-linux` (AT-SPI2)

### Tauri Application Flow

1. `src-tauri/src/lib.rs` sets up overlay window with click-through behavior
2. Global keyboard hooks via `rdev` crate capture home row keys
3. Frontend (`ui/main.ts`) renders hints at element positions
4. IPC commands: `get_elements`, `perform_action`, `set_click_through`, `check_permissions`

### Coordinate System Notes

- macOS AX API returns bottom-left origin coordinates; `scrim-macos` must convert to top-left
- Windows UIA returns physical pixels; divide by DPI scale factor for logical pixels
- All internal representations use top-left origin with logical pixels

## macOS Development

Accessibility permissions required:
1. System Settings > Privacy & Security > Accessibility
2. Add both Terminal (for CLI) and the built app binary

Check permissions: `cargo run -p scrim-cli -- permissions`

The app uses `macOSPrivateApi: true` in tauri.conf.json for click-through window support.
