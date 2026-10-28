# Hyprland

**English** · [Français](../fr/hyprland.md)

Two execution modes are available under Hyprland:

1. **Native Rust mode (`paros-rs`, recommended)**: Uses Wayland Layer Shell directly (`wlr-layer-shell`). **Zero window rules required**; pets float natively across all monitors in the `Overlay` layer.
2. **Standard Godot mode**: Uses separate XWayland windows and requires window rules plus a Python helper script (`linux/paros-hyprland.py`).

---

## 1. Native Rust Mode (`paros-rs` — Recommended)

The `paros-rs` binary communicates directly with the Hyprland UNIX IPC socket and Wayland Layer Shell:

```sh
# Run in background
./run-rs.sh -d

# Stop
./run-rs.sh -k
```

- **Zero configuration**: Nothing needed in `hyprland.conf`.
- **Native Mouse Passthrough**: Only pets and the menu capture clicks; all other screen areas pass clicks through without losing focus.
- **Automatic Multi-Monitor**: Queries monitor positions and geometries via `j/monitors`.
- **Terminal Double-Click**: Focuses the terminal window using `hyprctl dispatch focuswindow pid:<PID>`.
- See the [detailed paros-rs guide](paros-rs.md).

---

## 2. Standard Godot Mode (Legacy)

To run the Godot version under Hyprland:

### Window Rules
Hyprland tiles windows by default. To let Godot windows float freely without borders or shadows, add to `~/.config/hypr/hyprland.conf`:

```ini
windowrule = float 1, match:class ^(Paros)$
windowrule = pin 1, match:class ^(Paros)$
windowrule = border_size 0, match:class ^(Paros)$
windowrule = rounding 0, match:class ^(Paros)$
windowrule = no_shadow 1, match:class ^(Paros)$
windowrule = no_blur 1, match:class ^(Paros)$
windowrule = no_initial_focus 1, match:class ^(Paros)$
windowrule = no_anim 1, match:class ^(Paros)$

# Recommended if using hyprglass plugin:
windowrule = tag +hyprglass_disabled, match:class ^(Paros)$
```

Reload with `hyprctl reload`.

### Supported Features
- **Terminal focus**: double-clicking a pet raises its terminal window.
- **Perch**: pets perch on the active window returned by `hyprctl activewindow`.
- **Cursor tracking & idle**: cursor tracking via `hyprctl cursorpos`.
- **Full screen**: automatic avoidance of monitors running fullscreen apps.
- **Lock screen**: detection of `hyprlock` / `swaylock`.
