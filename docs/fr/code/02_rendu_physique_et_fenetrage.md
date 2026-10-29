# Rendu procédural, physique et multi-fenêtrage

Documentation technique du système graphique et physique de Paros.

---

## 1. Multi-fenêtrage transparent sous Godot 4

Paros n'utilise pas un overlay géant couvrant tous les moniteurs. Chaque personnage et chaque lettre volante possède sa propre fenêtre OS dédiée.

- **Configuration clé** :
  - `embed_subwindows = false` dans [`project.godot`](file:///home/user/Téléchargements/Paros_origin/project.godot).
  - Chaque instance de [`pet_window.tscn`](file:///home/user/Téléchargements/Paros_origin/src/pet/pet_window.tscn) hérite de `Window` natif (taille : 300x272 px).
  - Propriétés de fenêtre : `borderless = true`, `always_on_top = true`, `transparent = true`, `unfocusable = true`.
- **Fenêtre racine masquée** :
  - La fenêtre principale de Godot est déplacée hors de l'écran visible par [`pets.gd`](file:///home/user/Téléchargements/Paros_origin/src/pet/pets.gd) (`position = -size * 4`) pour ne pas intercepter la souris.
- **Désactivation de V-Sync** :
  - `DisplayServer.window_set_vsync_mode(VSYNC_DISABLED)` est appliqué sur chaque fenêtre. Sous Godot 4, des fenêtres transparentes multiples avec V-Sync bloquent le thread de rendu à 1 FPS.
- **Régulation du framerate** :
  - Contrôlé dynamiquement par `Engine.max_fps` dans [`pets.gd`](file:///home/user/Téléchargements/Paros_origin/src/pet/pets.gd) :
    - 30 FPS : au moins un pet bouge ou s'anime activement.
    - 12 FPS : tous les pets sont au repos (`IDLE` ou `SIT`).
    - 2 FPS : tous les pets sont masqués ou hors écran.

---

## 2. Gestionnaire de fenêtres ([`pets.gd`](file:///home/user/Téléchargements/Paros_origin/src/pet/pets.gd))

Classe `Pets` gérant le cycle de vie des fenêtres :

- `spawn(key, claude)` : Instancie une fenêtre [`pet_window.tscn`](file:///home/user/Téléchargements/Paros_origin/src/pet/pet_window.tscn) pour une session.
- `send_letter(from_pet, to_pet, message)` : Instancie ou recycle une fenêtre [`letter_window.tscn`](file:///home/user/Téléchargements/Paros_origin/src/pet/letter_window.tscn).
- `_spread()` : Algorithme répulsif évitant le chevauchement des mascottes immobiles.
- Supervision des interactions entre mascottes :
  - Saluts (`GREET`) quand deux mascottes se croisent.
  - High-fives quand deux sessions terminent avec succès dans un intervalle court.
  - Rivalités et regards noirs (`GLARE`) en cas de collision sur le même fichier.

---

## 3. Moteur cinématique et physique ([`pet.gd`](file:///home/user/Téléchargements/Paros_origin/src/pet/pet.gd))

Le nœud `Pet` ne déplace pas son sprite à l'intérieur du canvas : il déplace l'instance `Window` de l'OS sur le bureau.

### Système de coordonnées
- Le sol virtuel de la fenêtre se situe à `GROUND = Vector2(150, 246)`.
- La position logique du pet correspond à la coordonnée OS du point d'impact au sol de sa fenêtre.

### États physiques
- **Marche (`WALK`)** : Vitesse standard de 100 px/s (modifiée par `pace` selon la charge CPU). Vitesse accélérée (`HURRY` = 2.2x) pour revenir dans sa zone de repos.
- **Chute et gravité (`FALL`)** :
  - Accélération de gravité : `GRAVITY = 2600.0 px/s²`.
  - Rebond élastique : conservation de vitesse `BOUNCE = 0.5`.
  - Arrêt au sol lorsque la vitesse verticale descend sous `350 px/s`.
  - Parapluie : si la hauteur de chute dépasse 350 px, la vitesse verticale est bridée à `220 px/s`.
- **Portage à la souris (`CARRIED`)** :
  - Déclenché après un glissement supérieur à 6 px.
  - La fenêtre suit le curseur.
  - Vélocité lissée calculée en continu : devient la vitesse d'éjection balistique au lâcher (plafonnée à 3000 px/s).
- **Perchoir (`CLIMB`)** :
  - La mascotte peut grimper sur la bordure supérieure de la fenêtre applicative active.
  - Si la fenêtre active change de taille, de position ou se ferme, le pet bascule en chute libre.
- **Multi-écrans** :
  - Calcule la surface utile combinée (`_walk_area`) via `DisplayServer.screen_get_usable_rect()`.
  - Évite les moniteurs occupés par une application plein écran.

---

## 4. Rendu procédural vectoriel ([`pet_body.gd`](file:///home/user/Téléchargements/Paros_origin/src/pet/pet_body.gd))

Rendu 100 % vectoriel dans la méthode `_draw()` sans aucune texture externe.

### Grille et unités
- Basé sur une unité de pixel-art : `UNIT = 9.0 px`.
- Les proportions et membres sont calculés par rapport à cette grille.

### Types de mascottes
1. **Claude** :
   - Corps rectangulaire uni avec arrondi léger.
   - Yeux expressifs avec paupières mobiles et pupilles orientables vers le curseur de souris.
   - Membres articulés procéduralement (bras et jambes animés en sinusoïdes).
2. **Agy (Google Antigravity CLI)** :
   - Encodé sous forme de matrice texte 10x12 de codes hexadécimaux (`AGY_ROWS`).
   - Dessin matriciel par carrés de pixels colorés représentant la lettre « A » en dégradé bleu/cyan.
   - Deux jambes indépendantes animées lors de la marche.

### Animations mathématiques
- Aucune ressource `AnimationPlayer` : les mouvements reposent sur des fonctions trigonométriques du temps `t` écoulé.
  - Marche : oscillation verticale `abs(sin(t * 9 * pace))` et foulées des jambes `sin(t * 9 + phase)`.
  - Célébration (`CHEER`) : sauts de 18 px avec bras levés oscillants.
  - Tremblement (`WORRY`, `GLARE`) : vibration aléatoire haute fréquence (30 à 45 Hz).
  - Clignement des yeux : minuterie aléatoire entre 2 et 5 s, fermeture pendant 0.12 s.

### Accessoires et objets procéduraux
- Chapeaux et parures : haut-de-forme, couronne, casquette, nœud papillon, lunettes, casque de chantier, chapeau pointu et barbe du sage.
- Objets contextuels dessinés en 2D :
  - Feu de camp et guimauve lors de fortes charges CPU (`ROAST`).
  - Sablier animé avec écoulement de sable et retournement à 180° (`WAIT`).
  - Corbeille à papier et boulettes froissées lors de la compaction (`COMPACT`).
  - Balai et nuages de poussière lors d'un commit propre (`SWEEP`).
  - Fumée de saturation de contexte au-dessus de 75 % de tokens.

---

## 5. Interactions curseur et passthrough ([`pointer.gd`](file:///home/user/Téléchargements/Paros_origin/src/pet/pointer.gd))

Permet de cliquer à travers la fenêtre transparente sans bloquer l'OS.

### Transparence des clics (Click-through)
- **Linux (X11)** :
  - Définit un polygone de hit-test via `DisplayServer.window_set_mouse_passthrough()`.
  - Polygone de 24 sommets centré sur le corps du pet (`HIT_CENTER = Vector2(150, 192)`).
  - Tout clic en dehors de cette zone traverse directement vers les fenêtres sous-jacentes.
- **Windows** :
  - Définir un polygone découpe visuellement le rendu sous Windows.
  - Solution : bascule dynamique de la propriété booléenne `mouse_passthrough` à chaque frame selon `pet.hit_test(mouse_pos)`.

### Événements souris
- Clic simple : acclamation (`pointer_tap`).
- Double-clic : activation de la fenêtre du terminal (`pointer_double`).
- Clic droit : ouverture du menu contextuel ([`context_menu.gd`](file:///home/user/Téléchargements/Paros_origin/src/ui/context_menu.gd)).
- Glisser-déposer de fichiers : interception et copie du chemin dans le presse-papier ([`pointer_drop`](file:///home/user/Téléchargements/Paros_origin/src/pet/pointer.gd#L110)).

---

## 6. Courrier volant ([`letter.gd`](file:///home/user/Téléchargements/Paros_origin/src/pet/letter.gd))

- Fenêtre volante dédiée de 52x44 px ([`letter_window.tscn`](file:///home/user/Téléchargements/Paros_origin/src/pet/letter_window.tscn)).
- Déplacement balistique en arc parabolique depuis la main de l'émetteur jusqu'à la main du destinataire.
- Dessin vectoriel de l'enveloppe avec cachet de cire teinté à la couleur de la session émettrice.
- Recyclage : fenêtre masquée et parquée hors écran (`-20000, -20000`) une fois livrée.

---

## 7. Interface utilisateur

- **Bulles de parole ([`bubble.gd`](file:///home/user/Téléchargements/Paros_origin/src/ui/bubble.gd))** :
  - Dessinées au-dessus de la tête de la mascotte (`ANCHOR = Vector2(150, 132)`).
  - Affichage temporaire des messages de notification ou persistant pour la fiche d'information (`card`) au survol.
  - Calcul dynamique de boîte avec retour à la ligne automatique (`MAX_TEXT_WIDTH = 264 px`).
- **Menu contextuel ([`context_menu.gd`](file:///home/user/Téléchargements/Paros_origin/src/ui/context_menu.gd))** :
  - Menu surgissant `PopupMenu` accessible au clic droit.
  - Accès au terminal, localisation sonore, démarrage/arrêt du minuteur de focus, paramètres et quitter.
- **Fenêtre de réglages ([`settings_window.gd`](file:///home/user/Téléchargements/Paros_origin/src/ui/settings_window.gd))** :
  - Fenêtre standard Godot contenant onglets, cases à cocher et curseurs.
  - Synchronisation bidirectionnelle immédiate avec le singleton [`Settings`](file:///home/user/Téléchargements/Paros_origin/src/core/settings.gd).
