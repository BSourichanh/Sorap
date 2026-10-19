extends Node
## Follows the agy sessions (Google Antigravity CLI) running on this machine.
## Posts the session events of claude_code_sense.gd, the id of the agy
## conversation as session:
##   session_opened, session_changed {name, color, cwd, last_prompt, pid, context,
##     summary, kind: &"agy"}. name: title of the conversation (/rename, else
##     the one agy gives it), else the name of its folder. color and summary
##     stay empty: agy has neither,
##   session_closed,
##   session_phase {phase: &"idle" | &"working", since}. agy tells no wait for
##     the user that can be read,
##   session_activity {tool, detail}, session_quiet {level},
##   session_background {background, servers}: commands started in the
##     background that still run,
##   session_finished, session_tests_passed,
##   session_tool_failed {tool, detail, kind, agent}: a command that exited
##     with an error. agent is always empty,
##   session_turn {origin, from}: &"human", or &"peer" for a message of the
##     agent bridge (Diaulos), from naming its sender,
##   session_message_sent {to}: the session wrote to that address of the agent
##     bridge, such as "claude:<name>",
##   session_mail {mail}: messages of the agent bridge that wait in its inbox.
##
## Reads local files, none of them documented: may change.
## - /proc: each agy process holds the file presence/<conversation id>.lock of
##   its conversation open. Linux only: elsewhere no agy session is seen.
## - annotations/<conversation id>.pbtxt: the title of the conversation.
## - brain/<conversation id>/.system_generated/logs/transcript_full.jsonl: one
##   line per step, written once the step is done. The answer of the model with
##   its tool calls comes before their results. An answer with no tool call ends
##   the turn.
## - the log written by hooks/agy-hook.sh, one line per Stop: a turn that ends
##   cancelled or on an error has no last answer.
## - ~/.agent-bridge/inbox/agy-<conversation id>/: messages of the agent bridge.

## Fields of a hook log line, separated by tabs.
enum Field { EVENT, SESSION, REASON, COUNT }

const POLL_SECONDS := 0.5
## The processes are looked for less often: there are hundreds of them.
const SCAN_SECONDS := 2.0
## Name of the agy process in /proc/<pid>/comm.
const PROCESS := "agy"
const TRANSCRIPT := ".system_generated/logs/transcript_full.jsonl"
## A Stop of a turn that ended normally: its last answer told it already.
const NORMAL_END := "NO_TOOL_CALL"
## A first look at a session that seems to work: when its last step is older
## than this, its turn ended without a trace, cancelled before Paros started.
const STALE_SECONDS := 600.0
## Tool arguments that hold the path of the file a tool works on.
const PATH_ARGS: Array[String] = ["AbsolutePath", "TargetFile", "File", "DirectoryPath", "SearchPath"]
const COMMAND_TOOL := "run_command"
## Tokens are posted rounded to this step, to post less often.
const TOKEN_STEP := 10000
## Seconds without a new step that make a working session quiet at level 1, then 2.
const QUIET_SECONDS: Array[float] = [8.0, 25.0]
## Session fields that the pet shows: a change posts session_changed.
const SHOWN: Array[String] = ["name", "color", "cwd", "last_prompt", "pid", "context", "summary", "kind"]
const ClaudeCodeSense := preload("res://src/senses/claude_code_sense.gd")

var _agy_dir := Desktop.home().path_join(".gemini/antigravity-cli")
var _bridge_dir := Desktop.home().path_join(".agent-bridge")
var _proc_dir := "/proc"
var _log_path := ""
var _log_offset := 0
var _scan_in := 0.0
## Conversation id -> {pid, cwd}, as of the last scan.
var _live := {}
## Conversation id -> the SHOWN fields, plus working (true from the start of a
## turn to its last answer), phase, transcript_offset, known (false until its
## transcript was read once), calls (tool calls whose result is not read yet,
## oldest first, each {name, args}), tasks (background task id -> server),
## background, servers, mail, heard (time of its last sign of life), quiet,
## origin, asker, last_step (Unix time of the last step read).
var _sessions := {}
var _test_command := RegEx.create_from_string(
	"(^|[ /;&|])(pytest|jest|vitest|phpunit|rspec|ctest)([ ;&|]|$)|(npm|pnpm|yarn|bun|cargo|go|dotnet|mvn|gradle|make|composer)( run)? test")
var _exit_code := RegEx.create_from_string("The command exited with code (-?\\d+)")
var _task_started := RegEx.create_from_string("running as a background task with task id: (\\S+)")
var _task_ended := RegEx.create_from_string('Task id "([^"]+)" finished')
var _bridge_send := RegEx.create_from_string("bridge\\.py\"?\\s+send\\s+\"?([^\\s\"]+)")
var _sender := RegEx.create_from_string('<bridge-message [^>]*from="([^"]+)"')
var _request := RegEx.create_from_string("(?s)<USER_REQUEST>\\s*(.*?)\\s*</USER_REQUEST>")
var _title := RegEx.create_from_string('title:\\s*"((?:[^"\\\\]|\\\\.)*)"')


func _ready() -> void:
	_log_path = Desktop.runtime_dir().path_join("paros/agy-events.log")
	# Skip history: only hook events from now on matter.
	var file := FileAccess.open(_log_path, FileAccess.READ)
	if file:
		_log_offset = file.get_length()
	var timer := Timer.new()
	timer.wait_time = POLL_SECONDS
	timer.autostart = true
	timer.timeout.connect(_poll)
	add_child(timer)
	_poll()


func _poll() -> void:
	_scan_in -= POLL_SECONDS
	if _scan_in <= 0.0:
		_scan_in = SCAN_SECONDS
		_live = _read_processes()
		for id: String in _sessions.keys():
			if not _live.has(id):
				_sessions.erase(id)
				Events.post(&"session_closed", {"session": id})
	for id: String in _live:
		_update(id, _live[id])
	_read_hook_log()
	for id: String in _sessions:
		_update_mail(id)
		_update_phase(id)


## Conversation id -> {pid, cwd}, for each running agy process.
func _read_processes() -> Dictionary:
	var live := {}
	if not DirAccess.dir_exists_absolute(_proc_dir):
		return live
	var presence := _agy_dir.path_join("presence")
	for pid in DirAccess.get_directories_at(_proc_dir):
		if not pid.is_valid_int():
			continue
		var folder := _proc_dir.path_join(pid)
		if Desktop.read_kernel_file(folder.path_join("comm")).strip_edges() != PROCESS:
			continue
		var fds := DirAccess.open(folder.path_join("fd"))
		if fds == null:
			continue
		for fd in fds.get_files():
			var target := fds.read_link(fd)
			if target.get_base_dir() == presence and target.get_extension() == "lock":
				var cwd := DirAccess.open(folder)
				live[target.get_file().get_basename()] = {"pid": pid.to_int(), "cwd": cwd.read_link("cwd") if cwd else ""}
	return live


func _update(id: String, process: Dictionary) -> void:
	var opened := not _sessions.has(id)
	if opened:
		_sessions[id] = {
			"name": "", "color": "", "cwd": "", "last_prompt": "", "pid": 0, "context": 0, "summary": "", "kind": &"agy",
			"working": false, "phase": &"", "transcript_offset": 0, "known": false, "calls": [], "tasks": {},
			"background": 0, "servers": 0, "mail": 0, "heard": _now(), "quiet": 0, "origin": &"", "asker": "", "last_step": 0.0,
		}
	var session: Dictionary = _sessions[id]
	var before := _shown(session)
	session.pid = process.pid
	session.cwd = process.cwd
	if _scan_in == SCAN_SECONDS or opened:
		session.name = _read_title(id)
		if session.name.is_empty():
			session.name = session.cwd.get_file()
	_read_transcript(id, session)
	if not session.known:
		session.known = true
		# A turn told working with nothing new for long ended unseen.
		if session.working and Time.get_unix_time_from_system() - session.last_step > STALE_SECONDS:
			session.working = false
	var shown := _shown(session)
	if opened or shown != before:
		shown.session = id
		Events.post(&"session_opened" if opened else &"session_changed", shown)
	if opened and session.working and not session.origin.is_empty():
		_post_turn(id, session)
	var servers: int = session.tasks.values().count(true)
	var background: int = session.tasks.size() - servers
	if background != session.background or servers != session.servers:
		session.background = background
		session.servers = servers
		Events.post(&"session_background", {"session": id, "background": background, "servers": servers})


func _shown(session: Dictionary) -> Dictionary:
	var shown := {}
	for field in SHOWN:
		shown[field] = session[field]
	return shown


## Title of the conversation. agy writes it as text protobuf, with C escapes
## for quotes and for the bytes of the letters that are not ASCII.
func _read_title(id: String) -> String:
	var found := _title.search(FileAccess.get_file_as_string(_agy_dir.path_join("annotations/%s.pbtxt" % id)))
	if found == null:
		return ""
	var text := found.get_string(1)
	var bytes := PackedByteArray()
	var i := 0
	while i < text.length():
		var character := text[i]
		if character == "\\" and i + 1 < text.length():
			var octal := text.substr(i + 1, 3)
			if octal.length() == 3 and octal.is_valid_int() and not ("8" in octal or "9" in octal):
				bytes.append(_octal(octal))
				i += 4
				continue
			character = {"n": "\n", "t": "\t"}.get(text[i + 1], text[i + 1])
			i += 1
		bytes.append_array(character.to_utf8_buffer())
		i += 1
	return bytes.get_string_from_utf8()


func _octal(digits: String) -> int:
	var value := 0
	for digit in digits:
		value = value * 8 + digit.to_int()
	return value


## Reads only what was added to the transcript since the last call.
func _read_transcript(id: String, session: Dictionary) -> void:
	var file := FileAccess.open(_agy_dir.path_join("brain").path_join(id).path_join(TRANSCRIPT), FileAccess.READ)
	if file == null:
		return
	file.seek(session.transcript_offset)
	while file.get_position() < file.get_length():
		var line := file.get_line()
		# A line still being written has no end of line yet: read it whole next time.
		if file.eof_reached():
			break
		session.transcript_offset = file.get_position()
		var step: Variant = JSON.parse_string(line)
		if step is Dictionary:
			session.heard = _now()
			session.last_step = Time.get_unix_time_from_datetime_string(str(step.get("created_at", "")).trim_suffix("Z"))
			_read_step(id, session, step)


func _read_step(id: String, session: Dictionary, step: Dictionary) -> void:
	var content := str(step.get("content", ""))
	match step.get("type"):
		"USER_INPUT":
			session.working = true
			session.calls.clear()
			var sender := _sender.search(content)
			if sender:
				session.origin = &"peer"
				session.asker = sender.get_string(1)
			else:
				session.origin = &"human"
				session.asker = ""
				var request := _request.search(content)
				session.last_prompt = request.get_string(1) if request else content
			if session.known:
				_post_turn(id, session)
		"PLANNER_RESPONSE":
			var used: int = int(step.get("input_tokens", 0)) + int(step.get("cache_read_tokens", 0))
			if used > 0:
				session.context = roundi(float(used) / TOKEN_STEP) * TOKEN_STEP
			var calls: Array = step.get("tool_calls", []) if step.get("tool_calls") is Array else []
			if calls.is_empty():
				# The last answer of the turn.
				var was_working: bool = session.working
				session.working = false
				session.calls.clear()
				if session.known and was_working:
					Events.post(&"session_finished", {"session": id})
				return
			session.working = true
			for call: Variant in calls:
				if call is Dictionary:
					var args: Dictionary = call.get("args") if call.get("args") is Dictionary else {}
					session.calls.append({"name": str(call.get("name", "")), "args": args})
					if session.known:
						_read_call(id, str(call.get("name", "")), args)
		"GENERIC":
			if step.get("source") == "MODEL" and not session.calls.is_empty():
				_read_result(id, session, session.calls.pop_front(), content, step.get("status") == "RUNNING")
		"SYSTEM_MESSAGE":
			var ended := _task_ended.search(content)
			if ended and session.tasks.has(ended.get_string(1)):
				session.tasks.erase(ended.get_string(1))
				# agy hands the model the end of the task: a new turn of work.
				session.working = true


## A tool call of the model, before its result.
func _read_call(id: String, tool: String, args: Dictionary) -> void:
	var detail := ""
	for key in PATH_ARGS:
		if args.has(key):
			detail = str(args[key]).get_file()
			break
	if detail.is_empty():
		detail = str(args.get("toolSummary", ""))
	Events.post(&"session_activity", {"session": id, "tool": tool, "detail": detail})
	if tool == COMMAND_TOOL:
		var sent := _bridge_send.search(str(args.get("CommandLine", "")))
		if sent:
			Events.post(&"session_message_sent", {"session": id, "to": sent.get_string(1)})


## The result of a tool call. running: the tool goes on in the background.
func _read_result(id: String, session: Dictionary, call: Dictionary, content: String, running: bool) -> void:
	if call.name != COMMAND_TOOL:
		return
	var command := str(call.args.get("CommandLine", ""))
	if running:
		var started := _task_started.search(content)
		if started:
			session.tasks[started.get_string(1)] = ClaudeCodeSense.is_server(command)
		return
	var code := _exit_code.search(content)
	if code == null or not session.known:
		return
	var test := _test_command.search(command) != null
	if code.get_string(1) != "0":
		Events.post(&"session_tool_failed", {
			"session": id, "tool": COMMAND_TOOL, "detail": str(call.args.get("toolSummary", command.left(120))),
			"kind": "test" if test else "", "agent": "",
		})
	elif test:
		Events.post(&"session_tests_passed", {"session": id})


func _post_turn(id: String, session: Dictionary) -> void:
	Events.post(&"session_turn", {"session": id, "origin": session.origin, "from": session.asker})


func _read_hook_log() -> void:
	var file := FileAccess.open(_log_path, FileAccess.READ)
	if file == null:
		return
	if file.get_length() < _log_offset:
		_log_offset = 0
	file.seek(_log_offset)
	while file.get_position() < file.get_length():
		var fields := file.get_line().split("\t")
		if fields.size() == Field.COUNT and _sessions.has(fields[Field.SESSION]):
			var session: Dictionary = _sessions[fields[Field.SESSION]]
			session.heard = _now()
			# Cancelled, or an error: the turn ends with no last answer.
			if fields[Field.EVENT] == "Stop" and fields[Field.REASON] != NORMAL_END:
				session.working = false
				session.calls.clear()
	_log_offset = file.get_position()


func _update_mail(id: String) -> void:
	var session: Dictionary = _sessions[id]
	var inbox := _bridge_dir.path_join("inbox/agy-" + id)
	var mail := 0 if not DirAccess.dir_exists_absolute(inbox) else Array(DirAccess.get_files_at(inbox)).filter(func(file: String) -> bool: return file.get_extension() == "json").size()
	if mail != session.mail:
		session.mail = mail
		Events.post(&"session_mail", {"session": id, "mail": mail})


func _update_phase(id: String) -> void:
	var session: Dictionary = _sessions[id]
	var phase := &"working" if session.working else &"idle"
	if phase != session.phase:
		session.phase = phase
		session.heard = _now()
		Events.post(&"session_phase", {"session": id, "phase": phase, "since": Time.get_unix_time_from_system()})
	var silent_for: float = _now() - session.heard if phase == &"working" else 0.0
	var quiet := QUIET_SECONDS.filter(func(seconds: float) -> bool: return silent_for >= seconds).size()
	if quiet != session.quiet:
		session.quiet = quiet
		Events.post(&"session_quiet", {"session": id, "level": quiet})


func _now() -> float:
	return Time.get_ticks_msec() / 1000.0
