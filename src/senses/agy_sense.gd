extends Node
## Follows the agy sessions (Google Antigravity CLI) running on this machine.
## Posts the session events of claude_code_sense.gd, the id of the agy
## conversation as session:
##   session_opened, session_changed {name, color, cwd, last_prompt, pid, context,
##     summary, kind: &"agy"}. name: title of the conversation (/rename, else
##     the one agy gives it), else the name of its folder. color and summary
##     stay empty: agy has neither,
##   session_closed,
##   session_phase {phase: &"idle" | &"working" | &"waiting", since}. waiting:
##     the model asked the user a question (ask_question) and waits for the
##     answer. A wait for a permission is written nowhere,
##   session_needs_you {detail}: the question,
##   session_activity {tool, detail}, session_quiet {level},
##   session_background {background, servers}: commands and timers started in
##     the background that still run,
##   session_compacting {compacting, auto}: agy truncated the start of the
##     conversation, replaced by a summary. Told once done: both come one after
##     the other,
##   session_finished, session_tests_passed,
##   session_subagents {count, agents: [{id, name, tool, detail}]}: the
##     running subagents (invoke_subagent), oldest first, each named after the
##     role it was given, with the tool it uses,
##   session_tool_failed {tool, detail, test, agent}: a command that exited
##     with an error, or a tool that failed. agent: id of the subagent it
##     comes from, empty for the session,
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
##   its tool calls comes before their results, one step each. An answer with
##   no tool call ends the turn. A message of the system (end of a background
##   task or of a timer, report of a subagent) starts work again. Each
##   subagent has a conversation of its own, with its own transcript.
## - the log written by hooks/agy-hook.sh, one line per Stop: a turn that ends
##   cancelled or on an error has no last answer.
## - ~/.agent-bridge/inbox/agy-<conversation id>/: messages of the agent bridge.

## Fields of a hook log line, separated by tabs.
enum Field { EVENT, SESSION, NOTIFICATION, TOOL, DETAIL, KIND, AGENT, REASON, COUNT }

const POLL_SECONDS := 0.5
## The processes are looked for less often: there are hundreds of them.
const SCAN_SECONDS := 2.0
## Process names recognized for Antigravity (CLI and IDE/language_server).
const PROCESSES: Array[String] = ["agy", "language_server", "antigravity", "antigravity-ide"]
const TRANSCRIPT := ".system_generated/logs/transcript_full.jsonl"
## A Stop of a turn that ended normally: its last answer told it already.
const NORMAL_END := "NO_TOOL_CALL"
## A first look at a session that seems to work: when its last step is older
## than this, its turn ended without a trace, cancelled before Paros started.
const STALE_SECONDS := 600.0
## Title agy gives a conversation until it finds a better one.
const UNTITLED := "New Conversation"
## Tool arguments that hold the path of the file a tool works on, then those
## shown as they are: an address, a search.
const PATH_ARGS: Array[String] = ["AbsolutePath", "TargetFile", "File", "DirectoryPath", "SearchPath", "SearchDirectory"]
const TEXT_ARGS: Array[String] = ["Url", "query", "Query", "Pattern"]
const COMMAND_TOOL := "run_command"
## Asks the user, and waits for the answer.
const QUESTION_TOOL := "ask_question"
## Stops a task started in the background: its end is told nowhere else.
const TASK_TOOL := "manage_task"
const KILL_ACTION := "kill"
## Starts subagents. Its result gives the conversation id of each.
const SUBAGENT_TOOL := "invoke_subagent"
## In a message of the system: what it tells, then who sends it.
const MESSAGE_MARK := "[Message]"
## Step that replaces the start of a conversation grown too long with a summary.
const CHECKPOINT := "CHECKPOINT"
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
var _db_helper_script := "res://linux/paros-antigravity-db.py"
var _desktop_meta := {}
var _last_db_mtime := 0
## Conversation id -> {pid, cwd}, as of the last scan.
var _live := {}
## Conversation id -> the SHOWN fields, plus working (true from the start of a
## turn to its last answer), phase, transcript_offset, known (false until its
## transcript was read once), calls (tool calls whose result is not read yet,
## oldest first, each {name, args}), tasks (background task id -> server),
## background, servers, mail, heard (time of its last sign of life), quiet,
## origin, asker, last_step (Unix time of the last step read), subagents
## (conversation id -> {id, name, tool, detail, running, calls, tasks, offset,
## known}).
var _sessions := {}
var _test_command := RegEx.create_from_string(
	"(^|[ /;&|])(pytest|jest|vitest|phpunit|rspec|ctest)([ ;&|]|$)|(npm|pnpm|yarn|bun|cargo|go|dotnet|mvn|gradle|make|composer)( run)? test")
var _exit_code := RegEx.create_from_string("The command exited with code (-?\\d+)")
var _task_started := RegEx.create_from_string("running as a background task with task id: (\\S+)")
var _message_sender := RegEx.create_from_string("\\[Message\\][^\\n]* sender=(\\S+)")
var _subagent_id := RegEx.create_from_string('"conversationId":\\s*"([^"]+)"')
var _bridge_send := RegEx.create_from_string("bridge\\.py\"?\\s+send\\s+\"?([^\\s\"]+)")
var _sender := RegEx.create_from_string('<bridge-message [^>]*from="([^"]+)"')
var _request := RegEx.create_from_string("(?s)<USER_REQUEST>\\s*(.*?)\\s*</USER_REQUEST>")
var _canceled_subagent := RegEx.create_from_string('Subagent "([^"]+)" has been explicitly canceled')
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
		_sync_desktop_meta()
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


func _sync_desktop_meta() -> void:
	var db_path := ""
	for dir in _candidate_dirs():
		var p := dir.path_join("conversation_summaries.db")
		if FileAccess.file_exists(p):
			db_path = p
			break
	if db_path.is_empty():
		return
	var wal_path := db_path + "-wal"
	var mtime := FileAccess.get_modified_time(db_path)
	if FileAccess.file_exists(wal_path):
		mtime = max(mtime, FileAccess.get_modified_time(wal_path))
	if mtime == _last_db_mtime:
		return
	_last_db_mtime = mtime

	var script_path := ProjectSettings.globalize_path(_db_helper_script)
	if not FileAccess.file_exists(script_path):
		script_path = Desktop.home().path_join("Documents/Paros/linux/paros-antigravity-db.py")
	if not FileAccess.file_exists(script_path):
		return

	var output: Array = []
	var err := OS.execute("python3", ["-I", script_path, "--db", db_path], output)
	if err == 0 and not output.is_empty():
		var json: Variant = JSON.parse_string(output[0])
		if json is Dictionary:
			_desktop_meta = json


func _candidate_dirs() -> Array[String]:
	var dirs: Array[String] = []
	if not _agy_dir.is_empty():
		dirs.append(_agy_dir)
	if _proc_dir == "/proc":
		var antigravity_ide := Desktop.home().path_join(".gemini/antigravity")
		if not dirs.has(antigravity_ide):
			dirs.append(antigravity_ide)
		var snap_cli := Desktop.home().path_join("snap/antigravity-cli/common/.gemini/antigravity-cli")
		if not dirs.has(snap_cli):
			dirs.append(snap_cli)
	return dirs


func _is_subagent(id: String, data_dir: String) -> bool:
	if _desktop_meta.has(id):
		return not str(_desktop_meta[id].get("parent_id", "")).is_empty()
	for session: Dictionary in _sessions.values():
		if session.get("subagents", {}).has(id):
			return true
	if FileAccess.file_exists(data_dir.path_join("annotations/%s.pbtxt" % id)):
		return false
	for cand in _candidate_dirs():
		if FileAccess.file_exists(cand.path_join("annotations/%s.pbtxt" % id)):
			return false
	var log_dir := data_dir.path_join("brain").path_join(id).path_join(".system_generated/logs")
	var tpath := log_dir.path_join("transcript.jsonl")
	if not FileAccess.file_exists(tpath):
		tpath = log_dir.path_join("transcript_full.jsonl")
	if FileAccess.file_exists(tpath):
		var f := FileAccess.open(tpath, FileAccess.READ)
		if f:
			var first_line := f.get_line()
			if (first_line.contains("[Message]") and first_line.contains("sender=")) \
				or (first_line.contains('"source":"SYSTEM"') and not first_line.contains("<USER_REQUEST>")):
				return true
			if first_line.contains("<USER_REQUEST>") or first_line.contains('"type":"USER_INPUT"'):
				return false
	return false


## Conversation id -> {pid, cwd, data_dir, is_ide}, for each running agy process.
func _read_processes() -> Dictionary:
	var live := {}
	if not DirAccess.dir_exists_absolute(_proc_dir):
		return live
	var candidate_dirs := _candidate_dirs()
	var presences: Array[String] = []
	for dir in candidate_dirs:
		presences.append(dir.path_join("presence"))
	var running_pids := {}
	var ide_running := false
	var ide_pid := -1
	for pid in DirAccess.get_directories_at(_proc_dir):
		if not pid.is_valid_int():
			continue
		var folder := _proc_dir.path_join(pid)
		var comm := Desktop.read_kernel_file(folder.path_join("comm")).strip_edges()
		if comm not in PROCESSES:
			continue
		var ipid := pid.to_int()
		running_pids[ipid] = true
		if comm in ["language_server", "antigravity", "antigravity-ide"]:
			ide_running = true
			ide_pid = ipid
		var fds := DirAccess.open(folder.path_join("fd"))
		if fds == null:
			continue
		for fd in fds.get_files():
			var target := fds.read_link(fd)
			var base := target.get_base_dir()
			var idx := presences.find(base)
			if idx != -1 and target.get_extension() == "lock":
				var conv_id := target.get_file().get_basename()
				if _is_subagent(conv_id, candidate_dirs[idx]):
					continue
				var cwd := DirAccess.open(folder)
				var session_cwd: String = cwd.read_link("cwd") if cwd else ""
				if _desktop_meta.has(conv_id) and not str(_desktop_meta[conv_id].get("cwd", "")).is_empty():
					session_cwd = str(_desktop_meta[conv_id].get("cwd"))
				live[conv_id] = {
					"pid": ipid,
					"cwd": session_cwd,
					"data_dir": candidate_dirs[idx],
					"is_ide": comm != "agy",
				}
	# Also pick up running background conversations from Desktop SQLite
	if ide_running and ide_pid > 0:
		for cid: String in _desktop_meta:
			var active_dir := ""
			for dir in candidate_dirs:
				if DirAccess.dir_exists_absolute(dir.path_join("brain").path_join(cid)):
					active_dir = dir
					break
			if not active_dir.is_empty() and not live.has(cid) and _desktop_meta[cid].get("not_idle", false) and str(_desktop_meta[cid].get("parent_id", "")).is_empty():
				live[cid] = {
					"pid": ide_pid,
					"cwd": str(_desktop_meta[cid].get("cwd", "")),
					"data_dir": active_dir,
					"is_ide": true,
				}
	# Keep working background sessions alive, and keep open IDE sessions alive while IDE runs.
	for id: String in _sessions:
		if live.has(id):
			continue
		var session: Dictionary = _sessions[id]
		var subagents_running: bool = not _running(session).is_empty()
		var is_ide_session: bool = session.get("is_ide", false) or (ide_running and session.pid == ide_pid)
		var is_db_running: bool = _desktop_meta.get(id, {}).get("not_idle", false)
		var keep_ide_alive: bool = is_ide_session and ide_running and (Time.get_unix_time_from_system() - session.get("last_step", 0.0) < 3600.0)
		if (session.working or subagents_running or is_db_running or keep_ide_alive) and (session.pid in running_pids or Desktop.is_process_running(session.pid)):
			live[id] = {
				"pid": session.pid,
				"cwd": session.cwd,
				"data_dir": session.data_dir,
				"is_ide": is_ide_session,
			}
	return live


func _update(id: String, process: Dictionary) -> void:
	var opened := not _sessions.has(id)
	if opened:
		_sessions[id] = {
			"name": "", "color": "", "cwd": "", "last_prompt": "", "pid": 0, "context": 0, "summary": "", "kind": &"agy",
			"working": false, "phase": &"", "transcript_offset": 0, "known": false, "calls": [], "tasks": {}, "subagents": {},
			"background": 0, "servers": 0, "mail": 0, "heard": _now(), "quiet": 0, "origin": &"", "asker": "", "last_step": 0.0,
			"data_dir": "", "is_ide": false,
		}
	var session: Dictionary = _sessions[id]
	var before := _shown(session)
	session.pid = process.pid
	session.cwd = process.cwd
	session.data_dir = process.get("data_dir", _agy_dir)
	session.is_ide = process.get("is_ide", session.get("is_ide", false))
	if _scan_in == SCAN_SECONDS or opened:
		session.name = _read_title(id, session)
		if session.name in ["", UNTITLED]:
			session.name = session.cwd.get_file()
	_read_transcript(id, session)
	_read_subagents(id, session)
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
func _read_title(id: String, session: Dictionary = {}) -> String:
	if _desktop_meta.has(id) and not str(_desktop_meta[id].get("title", "")).is_empty():
		return str(_desktop_meta[id].get("title"))
	var base_dir: String = session.get("data_dir", _agy_dir) if not session.is_empty() else _agy_dir
	var found := _title.search(FileAccess.get_file_as_string(base_dir.path_join("annotations/%s.pbtxt" % id)))
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
	var base_dir: String = session.get("data_dir", _agy_dir)
	var file := FileAccess.open(base_dir.path_join("brain").path_join(id).path_join(TRANSCRIPT), FileAccess.READ)
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
				if session.cwd == Desktop.home() or session.cwd.is_empty():
					var ws_regex := RegEx.create_from_string("Active Workspaces:\\s*\\n\\s*-\\s*([^\\n]+)")
					var ws_match := ws_regex.search(content)
					if ws_match:
						session.cwd = ws_match.get_string(1).strip_edges()
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
					var tool := str(call.get("name", ""))
					session.calls.append({"name": tool, "args": args})
					if tool == TASK_TOOL and args.get("Action") == KILL_ACTION:
						session.tasks.erase(str(args.get("TaskId", "")))
					if session.known:
						_read_call(id, tool, args)
		CHECKPOINT:
			session.context = 0
			if session.known:
				Events.post(&"session_compacting", {"session": id, "compacting": true, "auto": true})
				Events.post(&"session_compacting", {"session": id, "compacting": false, "auto": true})
		"SYSTEM_MESSAGE":
			# The end of a background task or of a timer, the report of a
			# subagent: agy hands it to the model, a new turn of work.
			var canceled := _canceled_subagent.search(content)
			if canceled:
				var cid := canceled.get_string(1)
				if session.subagents.has(cid):
					session.subagents[cid].running = false
			var sender := _message_sender.search(content)
			if sender and step.get("source") == "SYSTEM":
				session.tasks.erase(sender.get_string(1))
				session.working = true
		_:
			# The result of a tool call: GENERIC, or for older versions of agy,
			# a type per tool, such as RUN_COMMAND or CODE_ACTION.
			if step.get("source") == "MODEL" and not session.calls.is_empty():
				_read_result(id, session, session.calls.pop_front(), step)


## A tool call of the model, before its result.
func _read_call(id: String, tool: String, args: Dictionary) -> void:
	Events.post(&"session_activity", {"session": id, "tool": tool, "detail": _detail(args)})
	if tool == QUESTION_TOOL:
		var questions: Variant = args.get("questions")
		var first: Variant = questions[0] if questions is Array and not questions.is_empty() else {}
		Events.post(&"session_needs_you", {"session": id, "detail": str(first.get("question", "")) if first is Dictionary else ""})
	if tool == COMMAND_TOOL:
		var sent := _bridge_send.search(str(args.get("CommandLine", "")))
		if sent:
			Events.post(&"session_message_sent", {"session": id, "to": sent.get_string(1)})


## What a tool call works on: a file, an address, a search, else its summary.
func _detail(args: Dictionary) -> String:
	for key in PATH_ARGS + TEXT_ARGS:
		if args.has(key):
			return str(args[key]).get_file() if key in PATH_ARGS else str(args[key])
	return str(args.get("toolSummary", ""))


## The result of a tool call, in the given step. Its status is RUNNING when
## the tool goes on in the background, ERROR when it failed.
func _read_result(id: String, session: Dictionary, call: Dictionary, step: Dictionary) -> void:
	var content := str(step.get("content", ""))
	var command := str(call.args.get("CommandLine", ""))
	if call.name == SUBAGENT_TOOL:
		_add_subagents(session, call.args, content)
	match step.get("status"):
		"RUNNING":
			var started := _task_started.search(content)
			if started:
				session.tasks[started.get_string(1)] = call.name == COMMAND_TOOL and ClaudeCodeSense.is_server(command)
			return
		"ERROR":
			if session.known:
				Events.post(&"session_tool_failed", {
					"session": id, "tool": call.name, "detail": str(step.get("error", "")).left(120), "test": false, "agent": "",
				})
			return
	if call.name != COMMAND_TOOL:
		return
	var code := _exit_code.search(content)
	if code == null or not session.known:
		return
	var test := _test_command.search(command) != null
	if code.get_string(1) != "0":
		Events.post(&"session_tool_failed", {
			"session": id, "tool": COMMAND_TOOL, "detail": str(call.args.get("toolSummary", command.left(120))),
			"test": test, "agent": "",
		})
	elif test:
		Events.post(&"session_tests_passed", {"session": id})


## Notes the subagents that the result of an invoke_subagent call created,
## named after the roles of its arguments, in the same order.
func _add_subagents(session: Dictionary, args: Dictionary, content: String) -> void:
	var roles: Array = args.get("Subagents") if args.get("Subagents") is Array else []
	var created := _subagent_id.search_all(content)
	for i in created.size():
		var role: Variant = roles[i] if i < roles.size() else {}
		var agent_id := created[i].get_string(1)
		session.subagents[agent_id] = {
			"id": agent_id, "name": str(role.get("Role", role.get("TypeName", ""))) if role is Dictionary else "",
			"tool": "", "detail": "", "running": false, "calls": [], "tasks": {}, "offset": 0, "known": session.known,
		}


## Reads what was added to the transcript of each subagent, and posts the
## running ones when they changed.
func _read_subagents(id: String, session: Dictionary) -> void:
	var base_dir: String = session.get("data_dir", _agy_dir)
	var before := _running(session)
	for agent: Dictionary in session.subagents.values():
		var transcript_path := base_dir.path_join("brain").path_join(agent.id).path_join(TRANSCRIPT)
		if agent.running and FileAccess.file_exists(transcript_path):
			var mtime := FileAccess.get_modified_time(transcript_path)
			if Time.get_unix_time_from_system() - mtime > 60.0:
				agent.running = false
		var file := FileAccess.open(transcript_path, FileAccess.READ)
		if file == null:
			continue
		file.seek(agent.offset)
		while file.get_position() < file.get_length():
			var line := file.get_line()
			if file.eof_reached():
				break
			agent.offset = file.get_position()
			var step: Variant = JSON.parse_string(line)
			if step is Dictionary:
				_read_subagent_step(id, agent, step)
		agent.known = true
	var after := _running(session)
	if after != before:
		Events.post(&"session_subagents", {"session": id, "count": after.size(), "agents": after})


func _read_subagent_step(id: String, agent: Dictionary, step: Dictionary) -> void:
	match step.get("type"):
		"PLANNER_RESPONSE":
			var calls: Array = step.get("tool_calls") if step.get("tool_calls") is Array else []
			# An answer with no tool call: its work is done, until it is given
			# more. Unless it waits for a task it started in the background.
			agent.running = not calls.is_empty() or not agent.tasks.is_empty()
			agent.calls.clear()
			for call: Variant in calls:
				if call is Dictionary:
					var args: Dictionary = call.get("args") if call.get("args") is Dictionary else {}
					agent.calls.append({"name": str(call.get("name", "")), "args": args})
			if not agent.calls.is_empty():
				agent.tool = agent.calls[0].name
				agent.detail = _detail(agent.calls[0].args)
		"USER_INPUT":
			agent.running = true
		"SYSTEM_MESSAGE":
			var sender := _message_sender.search(str(step.get("content", "")))
			if sender and step.get("source") == "SYSTEM":
				agent.tasks.erase(sender.get_string(1))
				agent.running = true
		_:
			if step.get("source") != "MODEL" or agent.calls.is_empty():
				return
			var call: Dictionary = agent.calls.pop_front()
			var started := _task_started.search(str(step.get("content", "")))
			if started and step.get("status") == "RUNNING":
				agent.tasks[started.get_string(1)] = true
			var code := _exit_code.search(str(step.get("content", "")))
			var failed: bool = step.get("status") == "ERROR" or (code != null and code.get_string(1) != "0")
			if failed and agent.known:
				Events.post(&"session_tool_failed", {"session": id, "tool": call.name, "detail": _detail(call.args), "test": false, "agent": agent.id})


## The running subagents, oldest first, each {id, name, tool, detail}.
func _running(session: Dictionary) -> Array:
	var running := []
	for agent: Dictionary in session.subagents.values():
		if agent.running:
			running.append({"id": agent.id, "name": agent.name, "tool": agent.tool, "detail": agent.detail})
	return running


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
		var line := file.get_line()
		if line.is_empty():
			continue
		var fields := line.split("\t")
		if fields.size() < 3:
			continue
		var event := fields[0]
		var conv_id := fields[1]

		var session: Dictionary = _sessions.get(conv_id, {})
		var subagent_parent_id := ""
		if session.is_empty():
			for sid: String in _sessions:
				if _sessions[sid].subagents.has(conv_id):
					subagent_parent_id = sid
					break

		if session.is_empty() and subagent_parent_id.is_empty():
			continue

		if not session.is_empty():
			session.heard = _now()
		else:
			_sessions[subagent_parent_id].heard = _now()

		if fields.size() >= 7:
			var notif := fields[2]
			var tool := fields[3]
			var detail := fields[4]
			var kind := fields[5]
			var agent_id := fields[6]
			var reason := fields[7] if fields.size() > 7 else ""

			match event:
				"PreToolUse":
					if not session.is_empty():
						session.working = true
						if tool == QUESTION_TOOL:
							session.calls.append({"name": tool, "args": {}})
							Events.post(&"session_needs_you", {"session": conv_id, "detail": detail})
						else:
							Events.post(&"session_activity", {"session": conv_id, "tool": tool, "detail": detail})
						_update_phase(conv_id)
					elif not subagent_parent_id.is_empty():
						var p_sess: Dictionary = _sessions[subagent_parent_id]
						var agent_entry: Dictionary = p_sess.subagents[conv_id]
						agent_entry.tool = tool
						agent_entry.detail = detail
						agent_entry.running = true
						Events.post(&"session_subagents", {"session": subagent_parent_id, "subagents": _running(p_sess)})
				"PostToolUse":
					if not session.is_empty():
						if kind == "test" and notif.is_empty():
							Events.post(&"session_tests_passed", {"session": conv_id})
						elif not notif.is_empty():
							Events.post(&"session_tool_failed", {
								"session": conv_id, "tool": tool, "detail": detail, "test": kind == "test", "agent": "",
							})
					elif not subagent_parent_id.is_empty():
						if not notif.is_empty():
							Events.post(&"session_tool_failed", {
								"session": subagent_parent_id, "tool": tool, "detail": detail, "test": false, "agent": conv_id,
							})
				"Stop":
					if not session.is_empty():
						session.working = false
						session.calls.clear()
						if reason != NORMAL_END:
							_update_phase(conv_id)
						else:
							Events.post(&"session_finished", {"session": conv_id})
		else:
			var reason := fields[2]
			if event == "Stop" and not session.is_empty() and reason != NORMAL_END:
				session.working = false
				session.calls.clear()
				_update_phase(conv_id)
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
	if session.working and session.calls.any(func(call: Dictionary) -> bool: return call.name == QUESTION_TOOL):
		phase = &"waiting"
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
