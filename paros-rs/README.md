# sorap (moteur natif Rust)

Implémentation native en Rust de Sorap pour Wayland et Hyprland.

Ce sous-projet réimplémente le compagnon de bureau Sorap en Rust pur, sans dépendance vers le moteur Godot. Il offre des performances optimales (consommation mémoire 72,3 Mo VmRSS dont 48,6 Mo de mémoire partagée `wl_shm`, utilisation CPU < 1%, 0% GPU), une compatibilité native multi-écrans via Wayland Layer Shell, et un suivi complet multi-sessions pour Claude Code et Antigravity (agy).

Pour une comparaison détaillée ligne par ligne avec la version Godot originelle (Paros), consultez la [Matrice Comparative d'Architecture (`compatch.md`)](../compatch.md).

---

## Fonctionnalités Clés

- **Overlay Wayland natif** : S'intègre directement via le protocole `zwlr_layer_shell_v1` en couche `Layer::Overlay`. Aucun cadre, bordure, ombre ni gestionnaire de fenêtres requis (`exclusive_zone = -1`).
- **Traversée des clics transparente (`wl_region`)** : Seules les mascottes, sous-agents, bulles et menus interceptent les clics ; le reste de l'écran laisse passer les interactions instantanément sans vol de focus (`KeyboardInteractivity::None`).
- **Pipeline vectoriel 2D logiciel pur (0% GPU)** : Rendu Tiny-Skia sur tampons ARGB32 linéaires avec cache de glyphes Fontdue pré-rastérisés et blitting direct en mémoire.
- **Support multi-écrans transparent** : Détecte dynamiquement l'ensemble des écrans connectés via `smithay-client-toolkit` et l'IPC Hyprland. Projection en repère absolu global.
- **Suivi multi-sessions complet** :
  - Détection automatique des sessions concurrentes Claude Code et Antigravity.
  - Reconstitution d'historique de sous-agents via parsing différentiel O(1) et synchronisation SQLite.
  - Espacement mutuel automatique au sol pour éviter les chevauchements.
- **Sous-agents autonomes et pyramides acrobatiques** :
  - Chaque sous-agent (`small_pets`) apparaît auprès de sa mascotte parente, affiche son outil en cours et joue (sauts, courses, pyramides bornées aux limites physiques d'écran).
- **Conseiller Sage et lettres volantes** :
  - Quand une session consulte l'advisor, un Sage entre en scène.
  - Simulation cinématique en mémoire d'enveloppes volantes en arc parabolique (vitesse 700 px/s, apogée 220 px) avec sceau coloré.
- **Interactions physiques complètes** :
  - Glisser-déposer individuel, saut au clic, lancer balistique avec gravité et `safe_clamp` anti-panique.
  - Double-clic focalisant instantanément le terminal associé via l'arbre généalogique `/proc/<pid>/status` et IPC Hyprland direct.
  - Menu contextuel natif au clic droit.

---

## Organisation de l'Architecture (53 Modules, < 500 lignes)

Le code est structuré selon les principes SOLID et GoF, avec une conformité volumétrique stricte (100% des fichiers sous le seuil maximal absolu de 500 lignes) :

| Sous-système | Répertoire / Modules | Rôle |
|---|---|---|
| **Point d'entrée & Orchestration** | [`src/main.rs`](src/main.rs), [`src/app/mod.rs`](src/app/mod.rs), [`src/app/engine.rs`](src/app/engine.rs) | Boucle événementielle 60 FPS, dispatching Wayland, gestion des signaux et cycle de vie |
| **Logique Applicative & Sociale** | [`src/app/sessions.rs`](src/app/sessions.rs), [`src/app/session_sync.rs`](src/app/session_sync.rs), [`src/app/compacting.rs`](src/app/compacting.rs), [`src/app/social.rs`](src/app/social.rs), [`src/app/social_pyramids.rs`](src/app/social_pyramids.rs), [`src/app/state.rs`](src/app/state.rs), [`src/app/state_spread.rs`](src/app/state_spread.rs), [`src/app/state_resolve.rs`](src/app/state_resolve.rs) | Gestion des sessions, synchronisation desktop, compactage de contexte, salutations, High-Five et pyramides |
| **Pipeline Graphique** | [`src/render/mod.rs`](src/render/mod.rs), [`src/render/draw_pet.rs`](src/render/draw_pet.rs), [`src/render/claude_body.rs`](src/render/claude_body.rs), [`src/render/agy_body.rs`](src/render/agy_body.rs), [`src/render/text.rs`](src/render/text.rs), [`src/render/postures.rs`](src/render/postures.rs), [`src/render/particles.rs`](src/render/particles.rs), [`src/render/overlays.rs`](src/render/overlays.rs), [`src/render/accessories.rs`](src/render/accessories.rs), [`src/app/render_pass.rs`](src/app/render_pass.rs) | Tracé vectoriel Tiny-Skia, cache de glyphes, blitting direct, particules, overlays, damage tracking et input regions |
| **Cinématique & Moteur Physique** | [`src/pet/mod.rs`](src/pet/mod.rs), [`src/pet/behavior.rs`](src/pet/behavior.rs), [`src/pet/physics.rs`](src/pet/physics.rs), [`src/pet/motion.rs`](src/pet/motion.rs), [`src/pet/interact.rs`](src/pet/interact.rs), [`src/pet/types.rs`](src/pet/types.rs) | Intégration Euler, machine d'états, trajectoires perchoir, rebonds, gravité, `safe_clamp` et protection anti-catapulte |
| **Surveillance Sessions & Données** | [`src/session/mod.rs`](src/session/mod.rs), [`src/session/agy.rs`](src/session/agy.rs), [`src/session/agy_desktop.rs`](src/session/agy_desktop.rs), [`src/session/agy_history.rs`](src/session/agy_history.rs), [`src/session/claude.rs`](src/session/claude.rs), [`src/session/claude_tools.rs`](src/session/claude_tools.rs), [`src/session/proc.rs`](src/session/proc.rs), [`src/session/proc_scan.rs`](src/session/proc_scan.rs), [`src/session/git.rs`](src/session/git.rs), [`src/session/mpris.rs`](src/session/mpris.rs) | Détection CLI agy et Antigravity Desktop, tail buffer glissant 128 Ko O(1), parsing différentiel, VFS `/proc` sans fork, git worktrees et MPRIS |
| **Intégration Système & Matériel** | [`src/hyprland.rs`](src/hyprland.rs), [`src/monitor.rs`](src/monitor.rs), [`src/system.rs`](src/system.rs), [`src/sound.rs`](src/sound.rs), [`src/app/pointer.rs`](src/app/pointer.rs), [`src/app/dnd.rs`](src/app/dnd.rs) | IPC Unix Hyprland avec cache trame, wlr-layer-shell, surveillance batterie/thermique, audio avec moissonnage zombie |
| **Interface Utilisateur & Configuration**| [`src/menu.rs`](src/menu.rs), [`src/card.rs`](src/card.rs), [`src/letter.rs`](src/letter.rs), [`src/settings/mod.rs`](src/settings/mod.rs), [`src/settings/ini.rs`](src/settings/ini.rs), [`src/settings/store.rs`](src/settings/store.rs), [`src/i18n.rs`](src/i18n.rs) | Menu contextuel, fiches au survol, lettres volantes, réglages persistants avec support agy Desktop et internationalisation bilingue |

---

## Validation et Non-Régression

- **Tests unitaires Rust** : `cargo test` valide **43 tests unitaires** (0 échec).
- **Tests Godot de référence** : `./test.sh` valide **136 tests de conformité** (0 échec).
- **Total combiné** : **179/179 tests validés** (100% de réussite).
- **Clippy** : `cargo clippy -- -D warnings` propre (0 avertissement).

---

## Prérequis et Compilation

### Dépendances Système
- Compilateur Rust (édition 2021 ou supérieure, via `rustup` ou paquet système)
- Bibliothèques Wayland de développement (`libwayland-client`)
- Police DejaVuSans ou Noto (résolution automatique sur Debian, Arch, Fedora, openSUSE, NixOS)
- `pipewire` / `pw-play` (optionnel, pour les effets sonores synthétisés)

### Compilation
```sh
cargo build --release --manifest-path paros-rs/Cargo.toml
strip paros-rs/target/release/paros-rs
```

---

## Lancement et Contrôle

Utilisez le script racine [`run-rs.sh`](../run-rs.sh) :

```sh
# Lancement en arrière-plan (recommandé)
./run-rs.sh -d

# Lancement interactif (avant-plan avec logs)
./run-rs.sh

# Recompilation et relance
./run-rs.sh -b -d

# Arrêt du démon en cours
./run-rs.sh -k
```
