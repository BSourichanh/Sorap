extends Node
## Root of the main scene. Chores of the start.


func _enter_tree() -> void:
	if InputMethod.leaving:
		for child in get_children():
			child.free()
		return
	if "--settings" in OS.get_cmdline_args() or "--settings" in OS.get_cmdline_user_args():
		for child in get_children():
			if child.name != "SettingsWindow":
				child.free()
		return
	var pid_path := Desktop.runtime_dir().path_join("paros/paros.pid")
	if FileAccess.file_exists(pid_path):
		var pid_str := FileAccess.get_file_as_string(pid_path).strip_edges()
		if pid_str.is_valid_int():
			var other_pid := pid_str.to_int()
			if other_pid != OS.get_process_id() and Desktop.is_process_running(other_pid):
				get_tree().quit()
				return
	DirAccess.make_dir_recursive_absolute(Desktop.runtime_dir().path_join("paros"))
	var file := FileAccess.open(pid_path, FileAccess.WRITE)
	if file:
		file.store_string(str(OS.get_process_id()) + "\n")


func _ready() -> void:
	if InputMethod.leaving:
		return
	if "--settings" in OS.get_cmdline_args() or "--settings" in OS.get_cmdline_user_args():
		# Déplacer la fenêtre racine 64x64 hors de l'écran pour ne pas gêner
		get_window().position = Vector2i(-10000, -10000)

		var win: Window = get_node_or_null("SettingsWindow")
		if win:
			win.initial_position = Window.WINDOW_INITIAL_POSITION_ABSOLUTE
			win.close_requested.connect(func(): get_tree().quit())
			win.visibility_changed.connect(func():
				if not win.visible:
					get_tree().quit()
			)

			Events.post(&"settings_requested")

			# Centrer explicitement sur l'écran où se trouve la souris
			var mpos := DisplayServer.mouse_get_position()
			var target_rect := DisplayServer.screen_get_usable_rect(0)
			for i in range(DisplayServer.get_screen_count()):
				var r := DisplayServer.screen_get_usable_rect(i)
				if r.has_point(mpos):
					target_rect = r
					break
			win.position = target_rect.position + (target_rect.size - win.size) / 2
		return
	if not InputMethod.leaving:
		Autostart.refresh()


func _notification(what: int) -> void:
	if "--settings" in OS.get_cmdline_args() or "--settings" in OS.get_cmdline_user_args():
		return
	if what == NOTIFICATION_WM_CLOSE_REQUEST or what == NOTIFICATION_PREDELETE:
		var pid_path := Desktop.runtime_dir().path_join("paros/paros.pid")
		if FileAccess.file_exists(pid_path):
			var pid_str := FileAccess.get_file_as_string(pid_path).strip_edges()
			if pid_str == str(OS.get_process_id()):
				DirAccess.remove_absolute(pid_path)
