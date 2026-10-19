#!/bin/sh
# Hook agy (Google Antigravity CLI) : ajoute une ligne par événement, lue par Paros.
# Champs séparés par des tabulations : événement, conversationId, terminationReason.
# À déclarer pour Stop : un tour annulé ou en erreur n'a pas de dernière réponse
# dans le transcript, seul ce hook dit qu'il est fini.
# Affiche {} (agy attend un objet JSON) et sort toujours avec 0 : ne peut pas
# perturber une session.

input=$(head -c 6000)

# Première valeur texte de la clé donnée.
field() {
	printf '%s' "$input" | grep -o "\"$1\"[[:space:]]*:[[:space:]]*\"[^\"]*\"" | head -n 1 | sed 's/^[^:]*:[[:space:]]*"\(.*\)"$/\1/'
}

event=${1:-Stop}
dir="${XDG_RUNTIME_DIR:-/tmp}/paros"
mkdir -p "$dir" 2>/dev/null
printf '%s\t%s\t%s\n' "$event" "$(field conversationId)" "$(field terminationReason)" >> "$dir/agy-events.log" 2>/dev/null
printf '{}\n'
exit 0
