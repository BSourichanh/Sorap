# Architecture technique de `paros-rs`

[English](../en/architecture-rs.md) · **Français**

Ce document détaille la conception logicielle et les flux d'exécution de l'implémentation native Rust `paros-rs`.

---

## 1. Principes et découpage SOLID

Le projet applique rigoureusement les principes de responsabilité unique (SRP) et de séparation des préoccupations :

```
                        ┌────────────────────────────────┐
                        │      Événements Système        │
                        │ (Claude logs, Locks agy, Git)  │
                        └───────────────┬────────────────┘
                                        ▼
                        ┌────────────────────────────────┐
                        │    SessionWatcher (session.rs) │
                        └───────────────┬────────────────┘
                                        │ poll_all()
                                        ▼
┌──────────────────┐    ┌────────────────────────────────┐    ┌─────────────────┐
│ HyprlandClient   ├───▶│           ParosApp             │◀───┤   Tiny-Skia     │
│ (IPC Socket)     │    │     (Boucle 60 FPS, SCTK)      │    │  (render.rs)    │
└──────────────────┘    └──────┬──────────┬──────────┬───┘    └─────────────────┘
                               │          │          │
                 ┌─────────────┘          │          └─────────────┐
                 ▼                        ▼                        ▼
        ┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
        │  card::Builder  │      │  menu::Context  │      │ monitor::Manager│
        │   (Fiches UI)   │      │    (Actions)    │      │ (Surfaces SHM)  │
        └─────────────────┘      └─────────────────┘      └─────────────────┘
```

### Modules du projet

| Module | Rôle technique | Dépendances |
|---|---|---|
| [`main.rs`](../../paros-rs/src/main.rs) | Initialisation de la boucle SCTK, orchestration des frames (60 FPS), distribution des événements Wayland. | SCTK, wayland-client |
| [`session.rs`](../../paros-rs/src/session.rs) | Surveillance des processus et fichiers d'état : Claude Code, Antigravity, Git CLI, D-Bus MPRIS. | std::fs, serde_json |
| [`pet.rs`](../../paros-rs/src/pet.rs) | Modèle physique de la mascotte : gravité, inertie, perchoir sur fenêtre active, automate d'états. | std::time |
| [`render.rs`](../../paros-rs/src/render.rs) | Dessin pixel-art pur en mémoire logicielle (`PixmapMut`) : Claude, Agy, Sage, boîtes aux lettres, bulles. | tiny-skia, fontdue |
| [`card.rs`](../../paros-rs/src/card.rs) | Construction textuelle isolée et testable des cartes d'information au survol (Hover Cards). | Aucun |
| [`menu.rs`](../../paros-rs/src/menu.rs) | Structure et hit-testing géométrique du menu contextuel, émission d'actions typées. | Aucun |
| [`monitor.rs`](../../paros-rs/src/monitor.rs) | Abstraction multi-écrans : cycle de vie des surfaces Layer Shell, calcul d'intersection, `damage_buffer`. | SCTK, wayland-client |
| [`letter.rs`](../../paros-rs/src/letter.rs) | Modélisation balistique des lettres volantes en arc parabolique avec oscillation angulaire. | Aucun |
| [`sound.rs`](../../paros-rs/src/sound.rs) | Déclenchement non-bloquant des effets sonores système via `pw-play`. | std::process |
| [`hyprland.rs`](../../paros-rs/src/hyprland.rs) | Client IPC UNIX direct communiquant avec le compositeur Hyprland via requêtes JSON (`j/activewindow`, etc.). | std::os::unix |

---

## 2. Intégration Wayland et gestion multi-écrans

### Protocole Layer Shell
`paros-rs` instancie une surface `LayerSurface` par moniteur (`wl_output`) en couche `Overlay`. Cette configuration garantit :
- L'absence de bordures ou décorations de fenêtre.
- Le maintien au premier plan visuel sans voler le focus du clavier.
- La persistance lors des bascules d'espaces de travail (workspaces).

### Transparence et traversée des clics (Mouse Passthrough)
Pour ne pas bloquer les interactions avec le bureau, `paros-rs` redéfinit dynamiquement la région d'entrée (`wl_region`) de chaque moniteur :
- Seules les boîtes englobantes des mascottes, de leurs sous-agents et du menu contextuel sont déclarées comme zones cliquables.
- Toute la surface restante est transparente aux événements souris, qui traversent directement vers les fenêtres sous-jacentes.

### Synchronisation multi-écrans
L'application unifie l'espace de bureau :
- Les coordonnées physiques `x` et `y` des mascottes sont globales.
- Lors du rendu, chaque moniteur projette les mascottes selon son décalage logique (`mon.x`, `mon.y`).
- Si aucune mascotte n'est présente sur un écran donné, la surface est vidée et aucun coût de composition n'est engendré.

---

## 3. Pipeline de rendu Tiny-Skia & Fontdue

- Le rendu est 100% logiciel et ne dépend pas d'OpenGL ou de Vulkan.
- Chaque pixel est écrit directement dans un tampon mémoire partagé (`SlotPool` SHM) attaché à la surface Wayland.
- `fontdue` rastérise la police DejaVuSans-Bold avec antialiasing sous-pixel pour les étiquettes et les fiches d'information.
- L'effacement complet via `damage_buffer(0, 0, width, height)` élimine toute rémanence visuelle (ghosting) lors des déplacements rapides.
