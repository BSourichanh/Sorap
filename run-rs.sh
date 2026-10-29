#!/bin/sh
# Lance Sorap (version native Rust).
# Options :
#   -d, --detach   Lance en arrière-plan (détaché).
#   -k, --kill     Arrête l'instance en cours d'exécution.
#   -b, --build    Recompile le binaire avant de lancer.

set -e

here="$(cd "$(dirname "$0")" && pwd)"

# Détermination du binaire : sorap (release/debug) avec repli transparent sur paros-rs
if [ -x "$here/paros-rs/target/release/sorap" ]; then
	bin="$here/paros-rs/target/release/sorap"
elif [ -x "$here/paros-rs/target/debug/sorap" ]; then
	bin="$here/paros-rs/target/debug/sorap"
elif [ -x "$here/paros-rs/target/release/paros-rs" ]; then
	bin="$here/paros-rs/target/release/paros-rs"
elif [ -x "$here/paros-rs/target/debug/paros-rs" ]; then
	bin="$here/paros-rs/target/debug/paros-rs"
else
	bin="$here/paros-rs/target/release/sorap"
fi

runtime_dir="${XDG_RUNTIME_DIR:-/tmp}/paros"
pid_file="$runtime_dir/paros-rs.pid"

detach=0
rebuild=0
kill_only=0

for arg in "$@"; do
	case "$arg" in
		-d|--detach)
			detach=1
			;;
		-k|--kill)
			kill_only=1
			;;
		-b|--build)
			rebuild=1
			;;
	esac
done

if [ "$kill_only" = 1 ]; then
	stopped=0
	if [ -f "$pid_file" ]; then
		pid=$(cat "$pid_file" 2>/dev/null || true)
		if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
			kill "$pid" 2>/dev/null || true
			for i in 1 2 3 4 5 6 7 8 9 10; do
				kill -0 "$pid" 2>/dev/null || break
				sleep 0.05
			done
			if kill -0 "$pid" 2>/dev/null; then
				kill -9 "$pid" 2>/dev/null || true
			fi
			rm -f "$pid_file"
			echo "Sorap (Rust) arrêté (PID $pid)."
			stopped=1
		fi
	fi
	if pkill -f "paros-rs/target/(release|debug)/(sorap|paros-rs)" 2>/dev/null; then
		stopped=1
	elif pkill -f "$bin" 2>/dev/null; then
		stopped=1
	fi
	rm -f "$pid_file"
	if [ "$stopped" = 0 ]; then
		echo "Aucune instance active trouvée ou processus arrêté."
	fi
	exit 0
fi

# Singleton : vérifier si une instance tourne déjà
if [ -f "$pid_file" ]; then
	existing_pid=$(cat "$pid_file" 2>/dev/null || true)
	if [ -n "$existing_pid" ] && kill -0 "$existing_pid" 2>/dev/null; then
		echo "Sorap (Rust) tourne déjà (PID $existing_pid)."
		exit 0
	fi
fi

# Garde session verrouillée : ne pas démarrer par-dessus l'écran de verrouillage
if [ -f "/tmp/hypr_locked" ] || pidof hyprlock >/dev/null; then
	echo "Session verrouillée (hyprlock actif) : lancement de Sorap différé."
	exit 0
fi

# Compiler si absent ou si demandé
if [ ! -x "$bin" ] || [ "$rebuild" = 1 ]; then
	echo "Compilation de sorap en mode release..."
	cargo build --release --manifest-path "$here/paros-rs/Cargo.toml"
	if [ -x "$here/paros-rs/target/release/sorap" ]; then
		bin="$here/paros-rs/target/release/sorap"
	fi
fi

mkdir -p "$runtime_dir" 2>/dev/null

if [ "$detach" = 1 ]; then
	if command -v setsid >/dev/null 2>&1; then
		setsid -f "$bin" >/dev/null 2>&1
	else
		nohup "$bin" >/dev/null 2>&1 &
	fi

	sleep 0.2
	pid=$(pgrep -f "$bin" | tail -n 1)
	if [ -n "$pid" ]; then
		echo "$pid" > "$pid_file"
		echo "Sorap (Rust) lancé en arrière-plan (PID $pid)."
	else
		echo "Sorap (Rust) lancé en arrière-plan."
	fi
	exit 0
fi

# Mode interactif avant-plan
echo "$$" > "$pid_file"
trap 'rm -f "$pid_file"' EXIT INT TERM
exec "$bin"
