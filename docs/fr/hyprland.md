# Hyprland

[English](../en/hyprland.md) · **Français**

Deux modes d'exécution sont disponibles sous Hyprland :

1. **Mode natif Rust (`paros-rs`, recommandé)** : Utilise directement Wayland Layer Shell (`wlr-layer-shell`). **Aucune règle de fenêtre n'est requise**, les mascottes flottent nativement sur tous vos écrans en couche `Overlay`.
2. **Mode Godot standard** : Utilise des fenêtres XWayland individuelles et requiert des règles de fenêtres et un script d'assistance Python (`linux/paros-hyprland.py`).

---

## 1. Mode natif Rust (`paros-rs` — Recommandé)

Le binaire `paros-rs` s'intègre directement avec le socket IPC d'Hyprland et le protocole Wayland Layer Shell :

```sh
# Lancer en arrière-plan
./run-rs.sh -d

# Arrêter
./run-rs.sh -k
```

- **Zéro configuration** : Aucun ajout dans `hyprland.conf`.
- **Mouse Passthrough natif** : Seules les mascottes et le menu interceptent les clics, tout le reste de l'écran est cliquable sans perte de focus.
- **Multi-écrans automatique** : Détecte les résolutions et positions d'écrans via `j/monitors`.
- **Double-clic terminal** : Focalise la fenêtre du terminal via `hyprctl dispatch focuswindow pid:<PID>`.
- Voir le [guide détaillé de paros-rs](paros-rs.md).

---

## 2. Mode Godot standard (Legacy)

Pour exécuter la version Godot sous Hyprland :

### Règles de fenêtres
Hyprland tuile les fenêtres par défaut. Pour que les fenêtres Godot flottent librement sans bordure ni ombre, ajoutez à `~/.config/hypr/hyprland.conf` :

```ini
windowrule = float 1, match:class ^(Paros)$
windowrule = pin 1, match:class ^(Paros)$
windowrule = border_size 0, match:class ^(Paros)$
windowrule = rounding 0, match:class ^(Paros)$
windowrule = no_shadow 1, match:class ^(Paros)$
windowrule = no_blur 1, match:class ^(Paros)$
windowrule = no_initial_focus 1, match:class ^(Paros)$
windowrule = no_anim 1, match:class ^(Paros)$

# Recommandé si le plugin hyprglass est utilisé :
windowrule = tag +hyprglass_disabled, match:class ^(Paros)$
```

Rechargez avec `hyprctl reload`.

### Fonctionnalités supportées
- **Terminal au premier plan** : double-clic sur un personnage pour focaliser sa fenêtre terminal.
- **Perchoir** : le personnage se perche sur la fenêtre active retournée par `hyprctl activewindow`.
- **Curseur et inactivité** : suivi du curseur avec `hyprctl cursorpos`.
- **Plein écran** : évitement automatique des moniteurs occupés par une fenêtre en plein écran.
- **Écran de verrouillage** : détection de `hyprlock` / `swaylock`.
