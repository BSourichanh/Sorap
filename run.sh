#!/bin/sh
# Lance Paros (version Rust par défaut, ou Godot avec --godot).
# Options :
#   -d, --detach   Lance en arrière-plan (détaché).
#   -k, --kill     Arrête toutes les instances actives de Paros.
#   -b, --build    Recompile le binaire avant de lancer.
#   --godot        Lance la version Godot au lieu de Rust.

here="$(cd "$(dirname "$0")" && pwd)"
runtime_dir="${XDG_RUNTIME_DIR:-/tmp}/paros"
pid_godot="$runtime_dir/paros.pid"
pid_rs="$runtime_dir/paros-rs.pid"

detach=0
kill_only=0
rebuild=0
use_godot=0
filtered_args=""

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
		--godot)
			use_godot=1
			;;
		*)
			filtered_args="$filtered_args $arg"
			;;
	esac
done

if [ "$kill_only" = 1 ]; then
	stopped=0

	# Arrêt de l'instance Rust
	if [ -f "$pid_rs" ]; then
		pid=$(cat "$pid_rs" 2>/dev/null || true)
		if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
			kill "$pid" 2>/dev/null || true
			for i in 1 2 3 4 5 6 7 8 9 10; do
				kill -0 "$pid" 2>/dev/null || break
				sleep 0.05
			done
			if kill -0 "$pid" 2>/dev/null; then
				kill -9 "$pid" 2>/dev/null || true
			fi
			rm -f "$pid_rs"
			echo "Paros (Rust) arrêté (PID $pid)."
			stopped=1
		fi
	fi
	if pkill -f "paros-rs/target/release/paros-rs" 2>/dev/null; then
		stopped=1
	fi
	rm -f "$pid_rs"

	# Arrêt de l'instance Godot
	if [ -f "$pid_godot" ]; then
		pid=$(cat "$pid_godot" 2>/dev/null || true)
		if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
			kill "$pid" 2>/dev/null || true
			for i in 1 2 3 4 5 6 7 8 9 10; do
				kill -0 "$pid" 2>/dev/null || break
				sleep 0.05
			done
			if kill -0 "$pid" 2>/dev/null; then
				kill -9 "$pid" 2>/dev/null || true
			fi
			rm -f "$pid_godot"
			echo "Paros (Godot) arrêté (PID $pid)."
			stopped=1
		fi
	fi
	pkill -f "godot.*Paros" 2>/dev/null || true
	rm -f "$pid_godot"

	if [ "$stopped" = 0 ]; then
		echo "Aucune instance active trouvée."
	fi
	exit 0
fi

# Par défaut : lancer la version Rust si disponible
if [ "$use_godot" = 0 ] && [ -f "$here/paros-rs/Cargo.toml" ]; then
	exec "$here/run-rs.sh" "$@"
fi

# Lancement Godot (si demandé explicitement avec --godot)
if [ -f "$pid_godot" ]; then
	existing_pid=$(cat "$pid_godot" 2>/dev/null || true)
	if [ -n "$existing_pid" ] && kill -0 "$existing_pid" 2>/dev/null; then
		echo "Paros (Godot) tourne déjà (PID $existing_pid)."
		exit 0
	fi
fi

export XMODIFIERS=@im=none
export PAROS_RESTARTED=1
godot="${GODOT_PATH:-godot}"

if [ "$detach" = 1 ]; then
	mkdir -p "$runtime_dir" 2>/dev/null
	cmd=""
	if command -v "$godot" >/dev/null 2>&1; then
		cmd="$godot --path $here"
	elif [ -x "$here/build/paros.x86_64" ]; then
		cmd="$here/build/paros.x86_64"
	fi

	if [ -z "$cmd" ]; then
		echo "Godot introuvable, et pas de binaire dans build/." >&2
		echo "Indiquer Godot : export GODOT_PATH=/chemin/vers/godot" >&2
		exit 1
	fi

	if command -v setsid >/dev/null 2>&1; then
		setsid -f $cmd $filtered_args >/dev/null 2>&1
	else
		nohup $cmd $filtered_args >/dev/null 2>&1 &
	fi

	sleep 0.3
	bg_pid=$(cat "$pid_godot" 2>/dev/null || true)
	if [ -n "$bg_pid" ]; then
		echo "Paros (Godot) lancé en arrière-plan (PID $bg_pid)."
	else
		echo "Paros (Godot) lancé en arrière-plan."
	fi
	exit 0
fi

if command -v "$godot" >/dev/null 2>&1; then
	exec "$godot" --path "$here" $filtered_args
fi
if [ -x "$here/build/paros.x86_64" ]; then
	echo "Godot introuvable : lancement du binaire build/paros.x86_64." >&2
	exec "$here/build/paros.x86_64" $filtered_args
fi
echo "Godot introuvable, et pas de binaire dans build/." >&2
exit 1
