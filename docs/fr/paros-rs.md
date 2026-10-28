# Guide de Paros en Rust (`paros-rs`)

[English](../en/paros-rs.md) · **Français**

`paros-rs` est la réimplémentation native de Paros écrite en Rust pour les environnements de bureau Wayland modernes (Hyprland, Sway, River, etc.).

---

## Pourquoi une version native Rust ?

Alors que la version standard s'appuie sur le moteur de jeu Godot 4.7, `paros-rs` a été développée pour répondre aux besoins spécifiques des compositeurs Wayland tuilants :

1. **Intégration Wayland Layer Shell** : L'application n'utilise aucune fenêtre conventionnelle sujette au tuilage ou aux décorations. Elle s'attache directement à la couche `Overlay` via le protocole `wlr-layer-shell-unstable-v1`.
2. **Aucune règle de fenêtrage requise** : Contrairement au binaire Godot qui nécessite des règles `windowrule = float`, `no_border`, `no_shadow`, `paros-rs` s'intègre nativement sans aucune configuration préalable.
3. **Empreinte de ressources minime** :
   - Mémoire vive : ~45 Mo (contre ~180-250 Mo pour Godot).
   - CPU : < 0,5% en marche, 0% au repos complet.
   - Binaire compilé : ~4 Mo après `strip`.
4. **Zéro fichier externe** : Le rendu pixel-art des mascottes, des accessoires, des bulles et des animations est généré procéduralement en mémoire par le moteur logiciel `tiny-skia`.

---

## Fonctionnalités principales

### Multi-sessions en temps réel
- Détection continue de toutes les sessions actives :
  - **Claude Code** : lecture des sessions dans `~/.claude/sessions/*.json` et des journaux d'événements `$XDG_RUNTIME_DIR/paros/claude-events.log`.
  - **Antigravity (agy)** : inspection des verrous de présence `~/.gemini/antigravity/presence/*.lock` et des descripteurs `/proc/<pid>/fd`.
- Chaque session ouverte possède sa propre mascotte adulte avec :
  - Son identifiant, son nom de session et sa branche Git courante.
  - Son accessoire déterministe (haut-de-forme, couronne, casquette, nœud, lunettes, fleur).
  - Ses fardeaux visuels : pile de dossiers selon le nombre de lignes sales non commitées, casque de chantier en cas de conflit de fusion/rebase, sablier en cas de tâche de fond, fumée en cas de contexte saturé.
- Espacement mutuel automatique au sol (`_spread`) pour éviter le chevauchement des mascottes.

### Sous-agents (`small_pets`)
- Chaque sous-agent lancé par une session parente instancie un petit personnage dédié.
- Il se positionne dans un périmètre restreint autour de sa mascotte parente (`home_reach`).
- Il affiche en continu son outil et son détail dans une bulle au-dessus de sa tête.
- À la fermeture du sous-agent, le personnage effectue un saut de joie avant de disparaître.

### Conseiller Sage et lettres volantes
- Quand une session consulte le conseiller (advisor Claude ou Sage), un personnage de Sage entre depuis le bord de l'écran.
- Des lettres volantes (`FlyingLetter`) sont propulsées en arc parabolique avec oscillation angulaire pour symboliser les échanges de messages inter-sessions et l'intervention du Sage.

### Contrôles et interactions
- **Clic gauche** : La mascotte saute de joie.
- **Glisser-déposer** : Attrapez n'importe quelle mascotte adulte pour la déplacer sur l'écran. En la lâchant avec élan, elle conserve son inertie et rebondit avec gravité.
- **Double-clic** : Focalise immédiatement la fenêtre de terminal exécutant la session associée via `hyprctl dispatch focuswindow pid:<PID>`.
- **Clic droit** : Ouvre le menu contextuel natif pour changer de mascotte (Claude / Agy), ouvrir les réglages ou quitter l'application.

---

## Utilisation

Le script [`run-rs.sh`](../../run-rs.sh) contrôle le cycle de vie du processus :

```sh
# Lancer en tâche de fond (daemon détaché)
./run-rs.sh -d

# Arrêter l'instance active
./run-rs.sh -k

# Recompiler et relancer en arrière-plan
./run-rs.sh -b -d
```
