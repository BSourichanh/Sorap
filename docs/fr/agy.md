# agy

[English](../en/agy.md)

Paros suit aussi les sessions d'agy, la CLI Antigravity de Google, pour l'utilisateur courant. Chacune a son personnage, dessiné d'après la mascotte d'agy : un « A » en dégradé, de l'orange au vert et au bleu. Il vit avec les personnages de Claude Code : il marche, s'assoit, dort, les salue, et ils s'écrivent des lettres par [Diaulos](https://github.com/Dr1mS/Diaulos).

![Un personnage Claude lance une lettre, un personnage agy en lit une, un autre réfléchit avec « run_command · Run tests » au-dessus de la tête, un dernier saute de joie](../images/agy.png)

Linux seulement. Rien à installer : Paros lit les fichiers qu'écrit agy. Un hook facultatif signale la fin d'un tour annulé.

## Ce que montre le personnage

| Session | Personnage |
|---|---|
| Ouverte | Un personnage apparaît, au nom de la conversation (`/rename`, sinon le titre que lui donne agy), à défaut au nom de son dossier. Son étiquette est bleue : agy n'a pas de `/color` |
| Au travail | Reste sur place, regarde en l'air, trois points se remplissent au-dessus de sa tête |
| Outil lancé | Légende au-dessus de la tête : « view_file · README.md », « run_command · Run tests » |
| Attend une commande lancée en arrière-plan | S'assoit et regarde un sablier |
| A laissé un serveur tourner en arrière-plan | Ses yeux brillent en vert |
| Fin d'un tour que tu as demandé | Saut, cœurs, bulle « Tâche finie ! » |
| Fin d'un tour demandé par une autre session | Hoche deux fois la tête |
| Tests réussis | Saut, bulle « Tests verts ! » |
| Tests échoués, autre commande en échec | Tremble, deux notes descendantes |
| Aucune nouvelle étape depuis 8 secondes, puis 25 | Tape du pied, puis médite |
| Écrit à une autre session par Diaulos | Lance une lettre, qui vole jusqu'au personnage de cette session |
| Un message de Diaulos attend dans sa boîte | Une boîte aux lettres se dresse à côté du personnage. Il lit la lettre quand la session la prend |
| Contexte rempli à 75 % ou plus | Sa tête fume. La fenêtre compte 1 million de jetons, celle des modèles Gemini |
| Fermée | Le personnage s'en va |

Ses bras n'apparaissent que lorsqu'ils se lèvent : quand il saute de joie, fait signe, lance ou lit. Au repos, le « A » garde la forme de la mascotte.

La branche git, les dossiers portés, le casque de chantier et les autres réactions au dépôt fonctionnent comme pour une session Claude Code : voir [Claude Code](claude-code.md).

## Diaulos

[Diaulos](https://github.com/Dr1mS/Diaulos) permet aux sessions agy et Claude Code de s'écrire. Paros montre ces messages comme des lettres :

- Une session Claude Code qui appelle l'outil `send` du serveur MCP `agent-bridge` lance une lettre au personnage de l'adresse.
- Une session agy qui lance `bridge.py send <adresse>` lance aussi une lettre.
- L'adresse désigne le personnage : `claude:<nom>`, `agy:<titre>` ou `agy:<début de l'id de conversation>`, `claude@<partie du dossier>`, `agy@<partie du dossier>`, ou `claude` ou `agy` seul quand une seule autre session de ce type tourne.
- Un message en attente dans la boîte d'une session agy (`~/.agent-bridge/inbox/agy-<id>/`) dresse une boîte aux lettres à côté de son personnage.

Rien à régler : Paros lit les commandes des sessions et les boîtes de Diaulos.

## Ce que lit Paros

| Fichier | Pour quoi |
|---|---|
| `/proc/<pid>/fd` de chaque processus `agy` | Le processus garde ouvert `~/.gemini/antigravity-cli/presence/<id de conversation>.lock` : la conversation qu'il fait tourner. Son dossier est celui du processus |
| `~/.gemini/antigravity-cli/annotations/<id de conversation>.pbtxt` | Le titre de la conversation |
| `~/.gemini/antigravity-cli/brain/<id de conversation>/.system_generated/logs/transcript_full.jsonl` | Une ligne par étape : la demande, chaque réponse du modèle avec ses appels d'outils, leurs résultats, la fin des tâches lancées en arrière-plan, les jetons utilisés |
| `~/.agent-bridge/inbox/agy-<id de conversation>/` | Les messages de Diaulos en attente |
| `$XDG_RUNTIME_DIR/paros/agy-events.log` | Écrit par le hook, voir plus bas |

Une étape est écrite une fois finie. La réponse du modèle vient avant les résultats de ses appels d'outils : la légende montre l'outil pendant qu'il tourne. Une réponse sans appel d'outil termine le tour.

## Le hook

Un tour annulé par l'utilisateur (Échap), ou arrêté sur une erreur, n'a pas de dernière réponse dans le transcript : sans le hook, le personnage continue de réfléchir, puis médite jusqu'au tour suivant. Le hook `Stop` d'agy le signale.

Dans `~/.gemini/config/hooks.json`, ajouter :

```json
{
  "paros": {
    "Stop": [
      { "type": "command", "command": "/chemin/vers/Paros/hooks/agy-hook.sh Stop", "timeout": 5 }
    ]
  }
}
```

Garder les autres clés du fichier, comme `agent-bridge` pour Diaulos. agy charge ses hooks au démarrage : relancer les sessions ouvertes.

Le script écrit une ligne par événement, trois champs séparés par des tabulations : l'événement, l'id de la conversation et la raison de la fin (`NO_TOOL_CALL`, `USER_CANCELED`, `ERROR`…). Il affiche `{}`, qui laisse agy s'arrêter, et sort toujours avec 0.

## Limites

- Linux seulement : la conversation d'un processus se lit dans `/proc`. Sous Windows, les sessions agy n'ont pas de personnage.
- La CLI agy seulement. Antigravity Desktop et l'IDE sont d'autres programmes : leurs sessions ne sont pas vues.
- agy n'écrit l'attente d'une permission dans aucun fichier que Paros peut lire : le personnage n'agite jamais les bras. Avec `toolPermission` à `always-proceed`, agy ne demande jamais.
- Une commande compte comme échouée quand son résultat indique « The command exited with code » suivi d'autre chose que 0. Une modification ou une lecture échouée ne montre rien.
- Un tour au travail sans nouvelle étape depuis 10 minutes au lancement de Paros est considéré comme fini.
- Les formats lus sont ceux d'agy avec le serveur de langage 1.4. Ils ne sont pas documentés et peuvent changer.

## Confidentialité

Paros lit les transcripts d'agy sur place, ne garde rien sur le disque et n'envoie rien. Le journal du hook ne contient que des noms d'événements, des ids de conversation et des raisons de fin. À l'écran, le personnage montre le titre de la conversation, la branche git et l'outil en cours ; la fiche au survol montre aussi le dossier et le début de la dernière demande.
