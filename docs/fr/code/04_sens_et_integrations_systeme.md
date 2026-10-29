# Sens et intégrations système

Documentation technique des capteurs d'environnement et des intégrations avec les outils externes dans Paros.

---

## 1. Architecture générale des sens (`src/senses/`)

Les sens sont des nœuds Godot spécialisés regroupés sous le nœud `Senses` dans [`main.tscn`](file:///home/user/Téléchargements/Paros_origin/main.tscn).

- **Principe** : Scruter l'environnement à intervalles réguliers et émettre des événements normalisés via `Events.post()`.
- **Préservation des performances** :
  - Les accès disques fréquents utilisent des pointeurs de fichier incrémentaux (offset mémorisé).
  - Les requêtes I/O potentiellement bloquantes (D-Bus, commandes Git, inspection de processus) sont exécutées dans des instances de `Thread` dédiées.

---

## 2. Détection de Claude Code ([`claude_code_sense.gd`](file:///home/user/Téléchargements/Paros_origin/src/senses/claude_code_sense.gd))

Paros combine quatre sources pour suivre fidèlement les sessions Claude Code :

### 1. Registre de sessions JSON
- **Chemin** : `~/.claude/sessions/<pid>.json` (ou `$CLAUDE_CONFIG_DIR/sessions`).
- **Contenu lu** : `sessionId`, répertoire de travail `cwd`, nom de session, statut (`idle`, `busy`, `waiting`, `shell`).
- **Contrôle de vivacité** : [`Desktop.is_process_running(pid)`](file:///home/user/Téléchargements/Paros_origin/src/core/desktop.gd#L84) valide l'existence réelle du processus pour ignorer les résidus de crash.

### 2. Lecture incrémentale des transcripts JSONL
- **Chemin** : `~/.claude/projects/*/<sessionId>.jsonl`.
- **Extraction continue** :
  - Consommation de tokens (`usage`).
  - Couleur attribuée via `/color` (`agent-color`).
  - Détection des sous-agents et tâches de fond (`backgroundTaskId`).
  - Détection de serveur actif (regex sur commandes de type `npm run dev`, `vite`, `python -m http.server`).
  - Conflits d'écriture concurrents (`session_collision`) : mise en relation des écritures récentes (`_written`) avec les erreurs d'outil `File has been modified since read`.
  - Début et fin des compactions de contexte (`compact_boundary`).

### 3. Hook shell synchrone ([`hooks/claude-hook.sh`](file:///home/user/Téléchargements/Paros_origin/hooks/claude-hook.sh))
- Configuré dans les hooks Claude Code (`PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `Notification`, `PreCompact`, `PostCompact`, `SubagentStart`, `SubagentStop`, `Stop`).
- Écrit une ligne TSV atomique dans `$XDG_RUNTIME_DIR/paros/claude-events.log`.
- Identifie en temps réel les exécutions de suites de tests (regex `pytest`, `cargo test`, `vitest`, `jest`…).

### 4. Espion de terminal AT-SPI ([`linux/paros-terminal.py`](file:///home/user/Téléchargements/Paros_origin/linux/paros-terminal.py))
- Script d'accessibilité GNOME/AT-SPI inspectant le contenu affiché dans les émulateurs de terminaux (`ptyxis`, `gnome-terminal`, `konsole`…).
- Détecte l'affichage de `"Advising using <model>"` afin de faire apparaître le Sage immédiatement pendant la consultation, avant même la fin de la requête réseau.

---

## 3. Détection d'Antigravity CLI / agy ([`agy_sense.gd`](file:///home/user/Téléchargements/Paros_origin/src/senses/agy_sense.gd))

Paros supporte nativement les sessions agy sur les bureaux Linux :

### 1. Découverte par descripteurs de verrous
- Parcours de `/proc/<pid>/fd/` toutes les 2 secondes.
- Localise les processus maintenant un verrou ouvert sur `~/.gemini/antigravity-cli/presence/<conversationId>.lock`.
- Extrait le PID et le répertoire courant depuis `/proc/<pid>/cwd`.

### 2. Métadonnées et annotations
- Nom et résumé : extraits de `~/.gemini/antigravity-cli/annotations/<conversationId>.pbtxt`.
- Gestion des séquences d'échappement octales Protobuf pour préserver les caractères accentués.

### 3. Transcripts JSONL agy
- Fichier : `~/.gemini/antigravity-cli/brain/<conversationId>/.system_generated/logs/transcript_full.jsonl`.
- Événements traités :
  - `USER_INPUT` : début de tour (`working = true`).
  - `PLANNER_RESPONSE` : relevé de tokens et des outils invoqués.
  - Outil `ask_question` : bascule instantanée en état d'attente (`waiting = true`, alerte mascotte).
  - Outil `manage_task` (action `kill`) : synchronisation de l'arrêt des processus de fond.
  - Outil `invoke_subagent` : enregistrement de l'identifiant du sous-agent et ouverture immédiate du transcript fils correspondant.

### 4. Boîte aux lettres Diaulos
- Surveille `~/.agent-bridge/inbox/agy-<id>/`.
- Déclenche l'envoi de lettres volantes dès qu'un message inter-agent est déposé.

### 5. Hook agy ([`hooks/agy-hook.sh`](file:///home/user/Téléchargements/Paros_origin/hooks/agy-hook.sh))
- Déclenché sur l'événement `Stop` pour signaler les interruptions utilisateur ou erreurs anormales.

---

## 4. Suivi de l'état Git ([`git_sense.gd`](file:///home/user/Téléchargements/Paros_origin/src/senses/git_sense.gd))

- Scrute les répertoires de travail des sessions actives toutes les 10 secondes.
- Exécute les commandes dans un thread dédié sans verrou bloquant :
  - `git --no-optional-locks -C <dossier> status --porcelain=v2 --branch` : extrait le hash de `HEAD`, la branche active, et le différentiel avec le remote (`ahead` / `behind`).
  - `git --no-optional-locks -C <dossier> diff --shortstat HEAD` : mesure le nombre de lignes modifiées.
- Détection directe des conflits en inspectant `.git/` (`MERGE_HEAD`, `rebase-merge`, `rebase-apply`, `CHERRY_PICK_HEAD`).
- Émet `repo_state` (branche, modifications, retard) et `repo_cleaned` (lorsqu'un commit rend le dépôt propre).

---

## 5. Détection multimédia et inactivité

### Musique ([`music_sense.gd`](file:///home/user/Téléchargements/Paros_origin/src/senses/music_sense.gd))
- **Linux (MPRIS / D-Bus)** :
  - Interroge le bus de session via `gdbus` en thread.
  - Filtre les services `org.mpris.MediaPlayer2.*` (Spotify, VLC, navigateurs web).
  - Lit la propriété `PlaybackStatus`. Dès qu'un lecteur est à `'Playing'`, émet `music_playing = true`.
- **Windows** :
  - Interroge l'API Windows Runtime SMTC (`GlobalSystemMediaTransportControlsSessionManager`) via le helper PowerShell.

### Inactivité utilisateur ([`idle_sense.gd`](file:///home/user/Téléchargements/Paros_origin/src/senses/idle_sense.gd))
- **Linux (GNOME Mutter)** :
  - Interroge l'interface D-Bus `org.gnome.Mutter.IdleMonitor` (`GetIdletime`).
- **Windows** :
  - Appel Win32 `GetLastInputInfo` exécuté par le script PowerShell.
- Émet `user_idle` ou `user_active` selon le seuil configuré (`sleep.idle_minutes`).

---

## 6. Métriques matérielles et horloge

### Système ([`system_sense.gd`](file:///home/user/Téléchargements/Paros_origin/src/senses/system_sense.gd))
- Exécuté toutes les 30 secondes.
- **Linux** :
  - Température CPU : `/sys/class/thermal/thermal_zone*/temp` (seuil critique : 92 °C).
  - Charge processeur : `/proc/loadavg` normalisé par le nombre de cœurs CPU (seuil de saturation : 0.9).
  - Batterie : `/sys/class/power_supply/*/capacity` et `status` (alerte en décharge sous 15 %).
- **Windows** : Lecture des moyennes dans `desktop.json`.
- Émet `cpu_hot`, `system_load`, `battery_low`.

### Horloge ([`clock_sense.gd`](file:///home/user/Téléchargements/Paros_origin/src/senses/clock_sense.gd))
- Contrôle périodique de l'heure locale.
- Émet `night` ou `day` selon la plage horaire configurée (`night_start_hour` / `night_end_hour`).

---

## 7. Extensions de bureau et scripts helpers

### Extension GNOME Shell ([`gnome-extension/paros@paros.local/`](file:///home/user/Téléchargements/Paros_origin/gnome-extension))
Indispensable sous Wayland pour franchir l'isolation des fenêtres :
- **Fichier d'état partagé** : Écrit `$XDG_RUNTIME_DIR/paros/desktop.json` (toutes les 500 ms) :
  - Position absolue du curseur souris (`global.get_pointer()`).
  - État de verrouillage de session (`Main.sessionMode.isLocked`).
  - Géométrie, PID et statut plein écran de la fenêtre active (`global.display.focus_window`).
  - Écrans recouverts par une fenêtre plein écran (`covered`).
- **Service D-Bus** : Expose la méthode `org.paros.Desktop.Activate(pids, title)` permettant à Paros de basculer le focus sur le bon terminal.
- **Miroir Lockscreen** : Clone visuellement les fenêtres des mascottes au-dessus du rideau de verrouillage pour garder un œil sur les sessions écran éteint.

### Helper Windows ([`windows/paros-desktop.ps1`](file:///home/user/Téléchargements/Paros_origin/windows/paros-desktop.ps1))
Script PowerShell assurant l'équivalent sous Windows :
- Surveillance via Win32 P/Invoke (`GetForegroundWindow`, `GetCursorPos`, `GetLastInputInfo`, `CreateToolhelp32Snapshot`).
- Écriture continue de `desktop.json` et `processes.json`.
- Traitement asynchrone des requêtes dans le dossier `requests/` :
  - `focus` : sélectionne l'onglet Windows Terminal correspondant via UI Automation (`SelectionItemPattern`) et force la mise au premier plan via `AttachThreadInput`.
  - `ring` : injecte le caractère sonore cloche `\a` dans `CONOUT$`.
