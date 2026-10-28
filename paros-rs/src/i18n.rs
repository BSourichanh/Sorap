#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum Lang {
    #[default]
    En,
    Fr,
}

pub type Language = Lang;


pub fn detect_lang(setting: &str) -> Lang {
    let normalized = setting.trim().to_ascii_lowercase();
    if normalized == "fr" || normalized.starts_with("fr_") || normalized.starts_with("fr-") {
        return Lang::Fr;
    }
    if normalized == "en" || normalized.starts_with("en_") || normalized.starts_with("en-") {
        return Lang::En;
    }

    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let v = val.trim().to_ascii_lowercase();
            if !v.is_empty() && v != "c" && v != "posix" {
                if v.starts_with("fr") {
                    return Lang::Fr;
                }
                return Lang::En;
            }
        }
    }

    Lang::En
}

pub fn tr(lang: Lang, text: &str) -> &str {
    match text {
        "%s needs you" | "%s a besoin de vous" => match lang {
            Lang::En => "%s needs you",
            Lang::Fr => "%s a besoin de vous",
        },
        "%s is waiting for you" | "%s vous attend" => match lang {
            Lang::En => "%s is waiting for you",
            Lang::Fr => "%s vous attend",
        },
        // Chaînes de src/core/language.gd
        "Task done!" | "task_done" => match lang {
            Lang::En => "Task done!",
            Lang::Fr => "Tâche finie !",
        },
        "Claude is waiting for you" | "claude_waiting" => match lang {
            Lang::En => "Claude is waiting for you",
            Lang::Fr => "Claude attend ta réponse",
        },
        "Claude has been waiting for %s" | "claude_waiting_for" => match lang {
            Lang::En => "Claude has been waiting for %s",
            Lang::Fr => "Claude attend depuis %s",
        },
        "%s: also changed by %s" | "also_changed_by" => match lang {
            Lang::En => "%s: also changed by %s",
            Lang::Fr => "%s : aussi modifié par %s",
        },
        "Green tests!" | "green_tests" => match lang {
            Lang::En => "Green tests!",
            Lang::Fr => "Tests verts !",
        },
        "Red tests" | "red_tests" => match lang {
            Lang::En => "Red tests",
            Lang::Fr => "Tests rouges",
        },
        "Focus: %d min" | "focus_min" => match lang {
            Lang::En => "Focus: %d min",
            Lang::Fr => "Focus : %d min",
        },
        "Break! %d min" | "break_min" => match lang {
            Lang::En => "Break! %d min",
            Lang::Fr => "Pause ! %d min",
        },
        "Break is over. Back to it?" | "break_over" => match lang {
            Lang::En => "Break is over. Back to it?",
            Lang::Fr => "Fin de pause. On reprend ?",
        },
        "Low battery: %d %%" | "low_battery" => match lang {
            Lang::En => "Low battery: %d %%",
            Lang::Fr => "Batterie faible : %d %%",
        },
        "Path copied: paste it in the terminal" | "path_copied" => match lang {
            Lang::En => "Path copied: paste it in the terminal",
            Lang::Fr => "Chemin copié : colle-le dans le terminal",
        },
        "Working for %s" | "working_for" => match lang {
            Lang::En => "Working for %s",
            Lang::Fr => "Travaille depuis %s",
        },
        "Working for" => match lang {
            Lang::En => "Working for",
            Lang::Fr => "Travaille depuis",
        },
        "Waiting for you for %s" | "waiting_for_you_for" => match lang {
            Lang::En => "Waiting for you for %s",
            Lang::Fr => "Attend ta réponse depuis %s",
        },
        "Waiting for you for" => match lang {
            Lang::En => "Waiting for you for",
            Lang::Fr => "Attend ta réponse depuis",
        },
        "At rest for %s" | "at_rest_for" => match lang {
            Lang::En => "At rest for %s",
            Lang::Fr => "Au repos depuis %s",
        },
        "At rest for" => match lang {
            Lang::En => "At rest for",
            Lang::Fr => "Au repos depuis",
        },
        "Waiting for a background task for %s" | "waiting_bg_for" => match lang {
            Lang::En => "Waiting for a background task for %s",
            Lang::Fr => "Attend une tâche en arrière-plan depuis %s",
        },
        "Waiting for a background task for" => match lang {
            Lang::En => "Waiting for a background task for",
            Lang::Fr => "Attend une tâche en arrière-plan depuis",
        },
        "Waiting for the answer of %s for %s" | "waiting_answer_for" => match lang {
            Lang::En => "Waiting for the answer of %s for %s",
            Lang::Fr => "Attend la réponse de %s depuis %s",
        },
        "Subagents running: %d" | "subagents_running" => match lang {
            Lang::En => "Subagents running: %d",
            Lang::Fr => "Sous-agents en cours : %d",
        },
        "Subagents running:" => match lang {
            Lang::En => "Subagents running:",
            Lang::Fr => "Sous-agents en cours :",
        },
        "Servers running: %d" | "servers_running" => match lang {
            Lang::En => "Servers running: %d",
            Lang::Fr => "Serveurs en marche : %d",
        },
        "Servers running:" => match lang {
            Lang::En => "Servers running:",
            Lang::Fr => "Serveurs en marche :",
        },
        "Letters to read: %d" | "letters_to_read" => match lang {
            Lang::En => "Letters to read: %d",
            Lang::Fr => "Lettres à lire : %d",
        },
        "Letters to read:" => match lang {
            Lang::En => "Letters to read:",
            Lang::Fr => "Lettres à lire :",
        },
        "Asks the sage" | "asks_the_sage" => match lang {
            Lang::En => "Asks the sage",
            Lang::Fr => "Demande au sage",
        },
        "Asks the sage for %s" | "asks_the_sage_for" => match lang {
            Lang::En => "Asks the sage for %s",
            Lang::Fr => "Demande au sage depuis %s",
        },
        "The sage comes when the advisor is asked" | "sage_comes" => match lang {
            Lang::En => "The sage comes when the advisor is asked",
            Lang::Fr => "Le sage vient quand l'advisor est consulté",
        },
        "Read the terminals to see the advisor at work" | "sage_live" => match lang {
            Lang::En => "Read the terminals to see the advisor at work",
            Lang::Fr => "Lire les terminaux pour voir l'advisor au travail",
        },
        "Context: %d k tokens (%d %%)" | "context_tokens" => match lang {
            Lang::En => "Context: %d k tokens (%d %%)",
            Lang::Fr => "Contexte : %d k tokens (%d %%)",
        },
        "Context:" => match lang {
            Lang::En => "Context:",
            Lang::Fr => "Contexte :",
        },
        "Compacting the context" | "compacting_context" => match lang {
            Lang::En => "Compacting the context",
            Lang::Fr => "Compactage du contexte",
        },
        "Context full: compacting" | "context_full_compacting" => match lang {
            Lang::En => "Context full: compacting",
            Lang::Fr => "Contexte plein : compactage",
        },
        "Context compacted" | "context_compacted" => match lang {
            Lang::En => "Context compacted",
            Lang::Fr => "Contexte compacté",
        },
        "Compacting the context for %s" | "compacting_context_for" => match lang {
            Lang::En => "Compacting the context for %s",
            Lang::Fr => "Compacte le contexte depuis %s",
        },
        "Compacting the context for" => match lang {
            Lang::En => "Compacting the context for",
            Lang::Fr => "Compacte le contexte depuis",
        },
        "Merge or rebase to finish" | "merge_or_rebase" => match lang {
            Lang::En => "Merge or rebase to finish",
            Lang::Fr => "Fusion ou rebase à terminer",
        },
        "Uncommitted: %d lines" | "uncommitted_lines" => match lang {
            Lang::En => "Uncommitted: %d lines",
            Lang::Fr => "Non commité : %d lignes",
        },
        "%d commits behind origin" | "commits_behind" => match lang {
            Lang::En => "%d commits behind origin",
            Lang::Fr => "En retard de %d commits sur origin",
        },
        "Focus: %d min left" | "focus_min_left" => match lang {
            Lang::En => "Focus: %d min left",
            Lang::Fr => "Focus : reste %d min",
        },
        "Break: %d min left" | "break_min_left" => match lang {
            Lang::En => "Break: %d min left",
            Lang::Fr => "Pause : reste %d min",
        },
        "“%s”" | "quote_fmt" => match lang {
            Lang::En => "“%s”",
            Lang::Fr => "« %s »",
        },

        // Menu contextuel (context_menu.gd + menu.rs)
        "Go to its terminal" | "Amener le terminal" | "menu_focus_terminal" => match lang {
            Lang::En => "Go to its terminal",
            Lang::Fr => "Aller à son terminal",
        },
        "Ring its terminal" | "Faire sonner le terminal" | "menu_ring_terminal" => match lang {
            Lang::En => "Ring its terminal",
            Lang::Fr => "Faire sonner son terminal",
        },
        "Start a focus (%d min)" | "menu_start_focus_fmt" => match lang {
            Lang::En => "Start a focus (%d min)",
            Lang::Fr => "Démarrer un focus (%d min)",
        },
        "Start a focus (25 min)" | "Minuteur de focus (25 min)" | "menu_start_focus" => match lang {
            Lang::En => "Start a focus (25 min)",
            Lang::Fr => "Démarrer un focus (25 min)",
        },
        "Stop the focus (%d min left)" | "menu_stop_focus_fmt" => match lang {
            Lang::En => "Stop the focus (%d min left)",
            Lang::Fr => "Arrêter le focus (reste %d min)",
        },
        "Stop the focus" | "Arrêter le minuteur de focus" | "menu_stop_focus" => match lang {
            Lang::En => "Stop the focus",
            Lang::Fr => "Arrêter le focus",
        },
        "Stop the break (%d min left)" | "menu_stop_break_fmt" => match lang {
            Lang::En => "Stop the break (%d min left)",
            Lang::Fr => "Arrêter la pause (reste %d min)",
        },
        "Stop the break" | "Arrêter la pause" | "menu_stop_break" => match lang {
            Lang::En => "Stop the break",
            Lang::Fr => "Arrêter la pause",
        },
        "Switch mascot (Claude / Agy)"
        | "Changer de mascotte (Claude / Agy)"
        | "menu_toggle_mascot" => match lang {
            Lang::En => "Switch mascot (Claude / Agy)",
            Lang::Fr => "Changer de mascotte (Claude / Agy)",
        },
        "Settings…" | "Réglages…" | "menu_settings" => match lang {
            Lang::En => "Settings…",
            Lang::Fr => "Réglages…",
        },
        "Close" | "Fermer" | "close" => match lang {
            Lang::En => "Close",
            Lang::Fr => "Fermer",
        },
        "Quit" | "Quitter" | "Quitter Paros" | "menu_quit" => match lang {
            Lang::En => "Quit",
            Lang::Fr => "Quitter",
        },

        // Fenêtre de réglages (settings_window.gd + language.gd)
        "Paros — Settings" | "Paros — Réglages" | "settings_title" => match lang {
            Lang::En => "Paros — Settings",
            Lang::Fr => "Paros — Réglages",
        },
        "Pet" | "Personnage" => match lang {
            Lang::En => "Pet",
            Lang::Fr => "Personnage",
        },
        "Size" | "Taille" => match lang {
            Lang::En => "Size",
            Lang::Fr => "Taille",
        },
        "Walk speed" | "Vitesse de marche" => match lang {
            Lang::En => "Walk speed",
            Lang::Fr => "Vitesse de marche",
        },
        "Show the session name" | "Afficher le nom de session" => match lang {
            Lang::En => "Show the session name",
            Lang::Fr => "Afficher le nom de session",
        },
        "Accessories" | "Accessoires" => match lang {
            Lang::En => "Accessories",
            Lang::Fr => "Accessoires",
        },
        "Pets greet each other" | "Les personnages se saluent" => match lang {
            Lang::En => "Pets greet each other",
            Lang::Fr => "Les personnages se saluent",
        },
        "Tower of resting sessions" | "Tour des sessions au repos" => match lang {
            Lang::En => "Tower of resting sessions",
            Lang::Fr => "Tour des sessions au repos",
        },
        "Headlamp at night" | "Lampe frontale la nuit" => match lang {
            Lang::En => "Headlamp at night",
            Lang::Fr => "Lampe frontale la nuit",
        },
        "Groove to the music" | "Bouger en musique" => match lang {
            Lang::En => "Groove to the music",
            Lang::Fr => "Bouger en musique",
        },
        "Sounds" | "Sons" => match lang {
            Lang::En => "Sounds",
            Lang::Fr => "Sons",
        },
        "Enable sounds" | "Activer les sons" => match lang {
            Lang::En => "Enable sounds",
            Lang::Fr => "Activer les sons",
        },
        "Volume" => "Volume",
        "Bubbles" | "Bulles" => match lang {
            Lang::En => "Bubbles",
            Lang::Fr => "Bulles",
        },
        "Show bubbles" | "Afficher les bulles" => match lang {
            Lang::En => "Show bubbles",
            Lang::Fr => "Afficher les bulles",
        },
        "Bubble duration" | "Durée des bulles" => match lang {
            Lang::En => "Bubble duration",
            Lang::Fr => "Durée des bulles",
        },
        "Claude Code" => "Claude Code",
        "Show the tool in use" | "Afficher l'outil en cours" => match lang {
            Lang::En => "Show the tool in use",
            Lang::Fr => "Afficher l'outil en cours",
        },
        "Subagents" | "Sous-agents" => match lang {
            Lang::En => "Subagents",
            Lang::Fr => "Sous-agents",
        },
        "A small pet for each subagent" | "Un petit personnage par sous-agent" => match lang {
            Lang::En => "A small pet for each subagent",
            Lang::Fr => "Un petit personnage par sous-agent",
        },
        "Small pets per session, at most" | "Petits personnages par session, au plus" => {
            match lang {
                Lang::En => "Small pets per session, at most",
                Lang::Fr => "Petits personnages par session, au plus",
            }
        }
        "Size of the small pets" | "Taille des petits personnages" => match lang {
            Lang::En => "Size of the small pets",
            Lang::Fr => "Taille des petits personnages",
        },
        "Show the tool of each subagent" | "Afficher l'outil de chaque sous-agent" => match lang {
            Lang::En => "Show the tool of each subagent",
            Lang::Fr => "Afficher l'outil de chaque sous-agent",
        },
        "Small pets jump and run to each other"
        | "Les petits personnages sautent et courent l'un vers l'autre" => match lang {
            Lang::En => "Small pets jump and run to each other",
            Lang::Fr => "Les petits personnages sautent et courent l'un vers l'autre",
        },
        "Small pets build pyramids" | "Les petits personnages font des pyramides" => match lang {
            Lang::En => "Small pets build pyramids",
            Lang::Fr => "Les petits personnages font des pyramides",
        },
        "Remind after waiting for" | "Insister après une attente de" => match lang {
            Lang::En => "Remind after waiting for",
            Lang::Fr => "Insister après une attente de",
        },
        "Knock when the terminal is not focused"
        | "Toquer quand le terminal n'a pas le focus" => match lang {
            Lang::En => "Knock when the terminal is not focused",
            Lang::Fr => "Toquer quand le terminal n'a pas le focus",
        },
        "Model context, in thousands of tokens"
        | "Contexte du modèle, en milliers de tokens" => match lang {
            Lang::En => "Model context, in thousands of tokens",
            Lang::Fr => "Contexte du modèle, en milliers de tokens",
        },
        "Follow the git status of folders" | "Suivre l'état git des dossiers" => match lang {
            Lang::En => "Follow the git status of folders",
            Lang::Fr => "Suivre l'état git des dossiers",
        },
        "Desktop (GNOME extension)" | "Bureau (extension GNOME)" => match lang {
            Lang::En => "Desktop (GNOME extension)",
            Lang::Fr => "Bureau (extension GNOME)",
        },
        "Desktop" | "Bureau" => match lang {
            Lang::En => "Desktop",
            Lang::Fr => "Bureau",
        },
        "Climb onto the focused window" | "Grimper sur la fenêtre active" => match lang {
            Lang::En => "Climb onto the focused window",
            Lang::Fr => "Grimper sur la fenêtre active",
        },
        "Sleep by the still pointer" | "Dormir contre le curseur immobile" => match lang {
            Lang::En => "Sleep by the still pointer",
            Lang::Fr => "Dormir contre le curseur immobile",
        },
        "Leave the screen of a full screen app"
        | "Quitter l'écran d'une app en plein écran" => match lang {
            Lang::En => "Leave the screen of a full screen app",
            Lang::Fr => "Quitter l'écran d'une app en plein écran",
        },
        "Screen on after locking" | "Écran allumé après verrouillage" => match lang {
            Lang::En => "Screen on after locking",
            Lang::Fr => "Écran allumé après verrouillage",
        },
        "Sleep" | "Sommeil" => match lang {
            Lang::En => "Sleep",
            Lang::Fr => "Sommeil",
        },
        "Night starts at" | "Début de la nuit" => match lang {
            Lang::En => "Night starts at",
            Lang::Fr => "Début de la nuit",
        },
        "Night ends at" | "Fin de la nuit" => match lang {
            Lang::En => "Night ends at",
            Lang::Fr => "Fin de la nuit",
        },
        "Sleep after inactivity" | "Sommeil après inactivité" => match lang {
            Lang::En => "Sleep after inactivity",
            Lang::Fr => "Sommeil après inactivité",
        },
        "Focus" => "Focus",
        "Focus duration" | "Durée d'un focus" => match lang {
            Lang::En => "Focus duration",
            Lang::Fr => "Durée d'un focus",
        },
        "Break duration" | "Durée d'une pause" => match lang {
            Lang::En => "Break duration",
            Lang::Fr => "Durée d'une pause",
        },
        "System" | "Système" => match lang {
            Lang::En => "System",
            Lang::Fr => "Système",
        },
        "Battery and temperature alerts" | "Alertes batterie et température" => match lang {
            Lang::En => "Battery and temperature alerts",
            Lang::Fr => "Alertes batterie et température",
        },
        "Start at login" | "Lancer au démarrage" => match lang {
            Lang::En => "Start at login",
            Lang::Fr => "Lancer au démarrage",
        },
        "Language" | "Langue" => match lang {
            Lang::En => "Language",
            Lang::Fr => "Langue",
        },
        "Automatic" | "Automatique" => match lang {
            Lang::En => "Automatic",
            Lang::Fr => "Automatique",
        },
        "English" => "English",
        "Français" => "Français",
        "Working..." | "Au travail..." => match lang {
            Lang::En => "Working...",
            Lang::Fr => "Au travail...",
        },
        "Question..." => "Question...",
        "CPU hot!" | "Processeur chaud !" => match lang {
            Lang::En => "CPU hot!",
            Lang::Fr => "Processeur chaud !",
        },
        _ => text,
    }
}
