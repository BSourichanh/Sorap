# Paros in Rust (`paros-rs`)

**English** · [Français](../fr/paros-rs.md)

`paros-rs` is the native Rust implementation of Paros designed for modern Wayland desktop environments (Hyprland, Sway, River, etc.).

---

## Why a native Rust implementation?

While the standard version relies on Godot 4.7, `paros-rs` was built specifically for tiling Wayland compositors:

1. **Wayland Layer Shell integration**: The app creates no standard windows subjected to tiling rules or window decorations. It attaches directly to the `Overlay` layer using `wlr-layer-shell-unstable-v1`.
2. **Zero window rules required**: Unlike the Godot engine which needs window rules (`windowrule = float`, `no_border`, `no_shadow`), `paros-rs` works out-of-the-box with no compositor configuration.
3. **Minimal resource footprint**:
   - RAM usage: ~45 MB (compared to ~180-250 MB for Godot).
   - CPU usage: < 0.5% walking, 0% when idle.
   - Binary size: ~4 MB stripped.
4. **Zero external assets**: Pixel-art graphics, accessories, bubbles, and UI cards are generated entirely in memory using the `tiny-skia` software rasterizer.

---

## Core Features

### Real-Time Multi-Session Tracking
- Continuous detection of all running sessions:
  - **Claude Code**: reads sessions from `~/.claude/sessions/*.json` and hook logs from `$XDG_RUNTIME_DIR/paros/claude-events.log`.
  - **Antigravity (agy)**: monitors locks in `~/.gemini/antigravity/presence/*.lock` and `/proc/<pid>/fd`.
- Each open session gets its own independent adult mascot with:
  - Session name and git branch.
  - Deterministic accessory (top hat, crown, cap, bow, glasses, flower).
  - Visual status: baggage stack for dirty lines, construction hard hat for merge/rebase conflicts, hourglass for background tasks, smoking head for context limits.
- Mutual ground spacing (`_spread`) prevents overlapping.

### Autonomous Subagents (`small_pets`)
- Every subagent spawned by a parent session gets its own small mascot.
- Follows the parent within a bounded area (`home_reach`).
- Permanently shows active tool and detail above its head.
- Performs a cheerful departure jump when completed.

### Sage Advisor & Flying Letters
- When a session consults the advisor model, a Sage enters from the screen edge.
- Parabolic flying envelopes (`FlyingLetter`) transport queries and replies between pets and the advisor.

### Controls & Gestures
- **Left click**: Mascot jumps with joy.
- **Drag & drop**: Pick up and throw any mascot; it flies with momentum and bounces upon landing.
- **Double click**: Instantly raises the session's terminal window via `hyprctl dispatch focuswindow pid:<PID>`.
- **Right click**: Native context menu to toggle mascot kind (Claude / Agy), open settings, or quit.

---

## Usage

Use the root control script [`run-rs.sh`](../../run-rs.sh):

```sh
# Launch as background daemon
./run-rs.sh -d

# Stop running daemon
./run-rs.sh -k

# Rebuild and restart
./run-rs.sh -b -d
```
