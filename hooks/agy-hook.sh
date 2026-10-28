#!/bin/sh
# Hook agy (Google Antigravity CLI & Desktop) : ajoute une ligne par événement, lue par Paros.
# Champs séparés par des tabulations :
#   Pour Stop : event, conversationId, terminationReason
#   Pour outils : event, conversationId, notification_type, tool_name, détail, genre, agent_id, reason
# N'affiche rien ou {} et sort toujours avec 0 : ne peut jamais perturber une session.

# Les champs utiles sont au début.
input=$(head -c 8000)

# Première valeur texte de la clé donnée (gère les guillemets échappés).
field() {
	printf '%s' "$input" | grep -oE "\"$1\"[[:space:]]*:[[:space:]]*\"(\\\\.|[^\\\\\"])*\"" | head -n 1 | sed -E 's/^[^:]*:[[:space:]]*"(.*)"$/\1/'
}

event=${1:-$(field hook_event_name)}
[ -z "$event" ] && event="Stop"

conv_id=$(field conversationId)
tool_name=$(field name)
error=$(field error)
reason=$(field terminationReason)

detail=$(field toolSummary)
[ -z "$detail" ] && detail=$(field CommandLine)
[ -z "$detail" ] && detail=$(field Path)
[ -z "$detail" ] && detail=$(field Role)
[ -z "$detail" ] && detail=$(field TypeName)
[ -z "$detail" ] && detail=$(field Recipient)

# Nettoyer les chemins (garder le nom de fichier).
detail=${detail##*/}
detail=${detail##*\\}

kind=""
if printf '%s' "$(field CommandLine)" | grep -qE '(^|[ /;&|])(pytest|jest|vitest|phpunit|rspec|ctest)([ ;&|]|$)|(npm|pnpm|yarn|bun|cargo|go|dotnet|mvn|gradle|make|composer)( run)? test'; then
	kind="test"
elif [ "$tool_name" = "invoke_subagent" ]; then
	kind="subagent"
fi

agent_id=""
if [ "$tool_name" = "send_message" ]; then
	agent_id=$(field Recipient)
fi

notif_type=""
if [ -n "$error" ]; then
	notif_type="error"
elif [ "$tool_name" = "ask_question" ]; then
	notif_type="permission_prompt"
fi

dir="${XDG_RUNTIME_DIR:-/tmp}/paros"
mkdir -p "$dir" 2>/dev/null

if [ "$event" = "Stop" ]; then
	printf '%s\t%s\t%s\n' "$event" "$conv_id" "${reason:-$error}" >> "$dir/agy-events.log" 2>/dev/null
else
	printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
		"$event" \
		"$conv_id" \
		"$notif_type" \
		"$tool_name" \
		"$(printf '%s' "$detail" | cut -c1-120 | tr '\t\n\r' '   ')" \
		"$kind" \
		"$agent_id" \
		"${reason:-$error}" >> "$dir/agy-events.log" 2>/dev/null
fi

if [ "$event" = "PreToolUse" ]; then
	printf '{"decision":"allow"}\n'
else
	printf '{}\n'
fi
exit 0
