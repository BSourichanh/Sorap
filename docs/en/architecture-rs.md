# Technical Architecture of `paros-rs`

**English** · [Français](../fr/architecture-rs.md)

This document describes the software architecture and runtime data flows of the native Rust implementation `paros-rs`.

---

## 1. SOLID Design & Modular Decomposition

The project enforces Single Responsibility (SRP) and separation of concerns:

```
                        ┌────────────────────────────────┐
                        │         System Events          │
                        │ (Claude logs, agy locks, Git)  │
                        └───────────────┬────────────────┘
                                        ▼
                        ┌────────────────────────────────┐
                        │    SessionWatcher (session.rs) │
                        └───────────────┬────────────────┘
                                        │ poll_all()
                                        ▼
┌──────────────────┐    ┌────────────────────────────────┐    ┌─────────────────┐
│ HyprlandClient   ├───▶│           ParosApp             │◀───┤   Tiny-Skia     │
│ (IPC Socket)     │    │     (60 FPS loop, SCTK)        │    │  (render.rs)    │
└──────────────────┘    └──────┬──────────┬──────────┬───┘    └─────────────────┘
                               │          │          │
                 ┌─────────────┘          │          └─────────────┐
                 ▼                        ▼                        ▼
        ┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
        │  card::Builder  │      │  menu::Context  │      │ monitor::Manager│
        │   (Hover Cards) │      │    (Actions)    │      │ (SHM Surfaces)  │
        └─────────────────┘      └─────────────────┘      └─────────────────┘
```

### Module Breakdown

| Module | Role | Dependencies |
|---|---|---|
| [`main.rs`](../../paros-rs/src/main.rs) | SCTK event loop bootstrap, 60 FPS frame orchestration, Wayland event dispatching. | SCTK, wayland-client |
| [`session.rs`](../../paros-rs/src/session.rs) | Session observation: Claude Code, Antigravity, Git status, MPRIS media state. | std::fs, serde_json |
| [`pet.rs`](../../paros-rs/src/pet.rs) | Pet physics: gravity, momentum, active window perching, state machine. | std::time |
| [`render.rs`](../../paros-rs/src/render.rs) | Pure pixel-art software rendering into `PixmapMut`: Claude, Agy, Sage, mailboxes, bubbles. | tiny-skia, fontdue |
| [`card.rs`](../../paros-rs/src/card.rs) | Isolated, unit-testable hover card text generation. | None |
| [`menu.rs`](../../paros-rs/src/menu.rs) | Geometric hit-testing and typed actions for native context menu. | None |
| [`monitor.rs`](../../paros-rs/src/monitor.rs) | Multi-monitor abstraction: LayerSurface lifecycle, intersection tests, `damage_buffer`. | SCTK, wayland-client |
| [`letter.rs`](../../paros-rs/src/letter.rs) | Ballistic flight modeling for flying letters with angular rocking. | None |
| [`sound.rs`](../../paros-rs/src/sound.rs) | Non-blocking system sound playback via `pw-play`. | std::process |
| [`hyprland.rs`](../../paros-rs/src/hyprland.rs) | Direct UNIX socket IPC client querying Hyprland (`j/activewindow`, etc.). | std::os::unix |

---

## 2. Wayland Integration & Multi-Monitor Support

### Layer Shell Protocol
`paros-rs` instantiates one `LayerSurface` per output (`wl_output`) attached to the `Overlay` layer. This guarantees:
- No window borders or decorations.
- Topmost visual presentation without stealing keyboard focus.
- Workspace switching immunity.

### Mouse Passthrough
To avoid obstructing desktop interactions:
- Only bounding boxes of adult pets, small pets, and the active context menu are registered in the Wayland input region (`wl_region`).
- All remaining surface space allows mouse clicks to pass through to underlying applications.

### Multi-Monitor Coordination
Desktop coordinates are globally unified:
- Pet positions `x` and `y` span across all connected screens.
- Rendering projects entities using each monitor's logical offset (`mon.x`, `mon.y`).
- Unoccupied monitors clear their buffer and avoid compositor redraw costs.

---

## 3. Tiny-Skia & Fontdue Rendering Pipeline

- 100% software rasterization with zero GPU driver dependencies.
- Pixels are drawn directly into shared memory buffers (`SlotPool` SHM) attached to Wayland surfaces.
- `fontdue` rasterizes DejaVuSans-Bold with subpixel antialiasing for text tags and hover cards.
- Full surface buffer damaging eliminates ghosting artifacts during fast ballistic throws.
