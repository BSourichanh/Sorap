# Cycle de vie des entités dans Paros

[English](../en/lifecycle.md) · **Français**

Ce document décrit le cycle de vie, la physique et les transitions d'état des mascottes adultes, des sous-agents, du conseiller Sage et des lettres volantes.

---

## 1. Mascottes adultes (`Pet`)

Chaque session active (Claude Code ou Antigravity) instancie une mascotte adulte unique.

```
       Session ouverte
              │
              ▼
         [Apparition] ──(sol)──▶ [Marche / Walk] ◀────┐
              │                         │             │
        (temps écoulé)            (repos inactif) (reprise)
              │                         │             │
              ▼                         ▼             │
         [Perchoir]               [Repos / Idle] ─────┘
      (active window)                   │
              │                 (inactivité longue)
              │                         ▼
              │                   [Sieste / Sit]
              ▼
   [Saisie / Drag & Drop] ──(lâcher avec élan)──▶ [Chute balistique] ──(sol)──▶ [Impact / Thud]
```

### États principaux
- **`Idle`** : La mascotte reste sur place, cligne des yeux (`blink_timer`), observe le curseur de souris si proche. Si la session est totalement au repos, elle allume un feu de camp (`campfire`).
- **`Walk`** : Déplacement autonome au sol avec dandinement physique (`leg_phase`). La vitesse est modulée par la charge de travail et les dossiers portés (`pace`).
- **`Perched`** : Si une fenêtre active Hyprland se trouve à proximité, la mascotte saute sur le bord supérieur de la fenêtre et s'y installe. Si la fenêtre bouge ou se ferme, elle tombe avec gravité.
- **`Carried`** : La mascotte est saisie à la souris. Le pointeur la déplace directement.
- **`Fall`** : Chute libre avec gravité (`vy += 980.0 * dt`). Si la hauteur de chute dépasse 200 px, elle ouvre un parapluie. À l'impact au sol, elle déclenche un son sourd (`thud`).
- **`Throw` / `Read`** : Postures d'envoi et de lecture de correspondance (lettres volantes).

---

## 2. Sous-agents (`small_pets`)

Lorsqu'une session lance des sous-agents via `invoke_subagent` (Antigravity) ou l'outil `Agent` / `Task` (Claude Code) :

1. **Naissance (`sync`)** :
   - Le sous-agent apparaît immédiatement aux pieds du parent.
   - Il effectue une course rapide initiale (`BIRTH_RUN` de 60 à 130 px à vitesse `pace = 2.2`) en tenant une feuille de papier (`sheet = true`).
2. **Périmètre orbital (`home_reach`)** :
   - Le sous-agent est rattaché à son parent via `parent_id`.
   - Sa zone de liberté (`home_reach = 140 + 30 * nb_agents`) suit automatiquement le déplacement de la mascotte adulte.
3. **Animations et jeux** :
   - Petits sauts de cabri réguliers.
   - Visites entre sous-agents voisins.
   - Construction de pyramides acrobatiques collectives.
4. **Départ (`_dismiss`)** :
   - Dès que le sous-agent termine sa tâche, il exécute un saut de joie (`cheer`), range sa feuille et court rapidement vers son parent.
   - Il disparaît à son arrivée ou après un délai de grâce de 6 secondes.

---

## 3. Conseiller Sage (`new_sage`)

Quand une session déclenche une demande d'assistance au modèle advisor :

1. **Arrivée** :
   - Un Sage vénérable (stature 1,5×, chapeau pointu, barbe blanche, bâton de pèlerin) entre depuis le bord de l'écran le plus proche.
   - Il court (`pace = 2.0`) jusqu'à s'arrêter à environ 230 px de la mascotte concernée.
2. **Échange de courrier** :
   - La mascotte parente se tourne vers lui et lui lance une lettre volante (`FlyingLetter`).
   - Le Sage lit la lettre (`State::Read`).
3. **Départ** :
   - Lorsque la réponse de l'advisor est prête, le Sage renvoie une lettre réponse vers la mascotte.
   - Le Sage salue et quitte l'écran vers le bord opposé, puis est libéré de la mémoire.

---

## 4. Lettres volantes (`FlyingLetter`)

Les lettres assurent la visualisation poétique de la communication inter-sessions et avec le conseiller :

- **Trajectoire balistique** : Arc parabolique calculé dynamiquement (`arc = min(distance * 0.25, 220.0)`).
- **Oscillation angulaire** : L'enveloppe tangue en vol (`angle = sin(time * 9.0) * 0.18`).
- **Sceau de cire déterministe** : Sceau coloré selon la couleur de la mascotte émettrice (Orange Claude `#d97757` ou Bleu Agy `#4a80ea`).
- **Atterrissage** :
  - Si le destinataire est libre, il prend la lettre en main pour la lire.
  - Si le destinataire est déjà occupé, la lettre se dépose dans sa boîte aux lettres (`mailbox`) avec compteur de courrier en attente.
