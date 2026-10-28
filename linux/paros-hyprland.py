#!/usr/bin/env python3
"""Hyprland integration helper for Paros.

Acts as the Hyprland counterpart of the GNOME Shell extension.
- Writes $XDG_RUNTIME_DIR/paros/desktop.json with focused window geometry (for perch),
  cursor position, covered screens (fullscreen detection), lock status, and idle_ms.
- Exports D-Bus org.gnome.Shell /org/paros/Desktop Activate(pids, title)
  so double-clicking a pet raises its session terminal in Hyprland.
- Uses native Hyprland UNIX socket IPC for minimal CPU and latency overhead.
- Terminates automatically when the parent Paros process ends.
"""
import argparse
import atexit
import json
import os
import signal
import socket
import subprocess
import sys
import time

try:
    import gi
    from gi.repository import Gio, GLib
except ImportError:
    gi = None

POLL_MS = 500
HEARTBEAT_MS = 10000

DBUS_XML = """
<node>
  <interface name="org.paros.Desktop">
    <method name="Activate">
      <arg type="au" direction="in" name="pids"/>
      <arg type="s" direction="in" name="title"/>
      <arg type="b" direction="out" name="found"/>
    </method>
  </interface>
</node>"""


def get_runtime_dir() -> str:
    return os.environ.get("XDG_RUNTIME_DIR", "/tmp")


class ParosHyprlandHelper:
    def __init__(self, parent_pid: int = 0):
        self.parent_pid = parent_pid
        self.runtime_dir = os.path.join(get_runtime_dir(), "paros")
        os.makedirs(self.runtime_dir, exist_ok=True)
        self.desktop_json_path = os.path.join(self.runtime_dir, "desktop.json")
        self.last_written = ""
        self.last_written_at = 0.0
        self.last_activity_at = time.monotonic()
        self.last_pointer = [0, 0]
        self.last_active_pid = -1
        self._last_lock_check = 0.0
        self._locked = False
        self.running = True

        sig = os.environ.get("HYPRLAND_INSTANCE_SIGNATURE", "")
        self.socket_path = os.path.join(get_runtime_dir(), "hypr", sig, ".socket.sock") if sig else ""

        self._setup_dbus()

    def _hypr_cmd(self, cmd: str) -> str:
        """Query Hyprland via native UNIX socket, falling back to hyprctl."""
        if self.socket_path and os.path.exists(self.socket_path):
            try:
                s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                s.settimeout(0.5)
                s.connect(self.socket_path)
                s.sendall(cmd.encode("utf-8"))
                chunks = []
                while True:
                    data = s.recv(4096)
                    if not data:
                        break
                    chunks.append(data)
                s.close()
                return b"".join(chunks).decode("utf-8", errors="replace")
            except Exception:
                pass

        # Fallback to subprocess hyprctl
        args = ["hyprctl"]
        if cmd.startswith("j/"):
            args.extend([cmd[2:], "-j"])
        elif cmd.startswith("dispatch "):
            args.extend(["dispatch"] + cmd[len("dispatch "):].split())
        else:
            args.extend(cmd.split())
        try:
            return subprocess.check_output(args, timeout=1).decode("utf-8", errors="replace")
        except Exception:
            return ""

    def _setup_dbus(self):
        if not gi:
            return
        try:
            self.bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
            node_info = Gio.DBusNodeInfo.new_for_xml(DBUS_XML)
            self.interface_info = node_info.interfaces[0]

            self.bus.register_object(
                "/org/paros/Desktop",
                self.interface_info,
                self._handle_dbus_call,
                None,
                None,
            )

            Gio.bus_own_name_on_connection(
                self.bus,
                "org.gnome.Shell",
                Gio.BusNameOwnerFlags.NONE,
                None,
                None,
            )
        except Exception as e:
            sys.stderr.write(f"Paros Hyprland D-Bus warning: {e}\n")

    def _handle_dbus_call(self, conn, sender, object_path, interface_name, method_name, parameters, invocation):
        if method_name == "Activate":
            pids, title = parameters.unpack()
            found = self.activate_window(pids, title)
            invocation.return_value(GLib.Variant("(b)", (found,)))

    def activate_window(self, pids, title: str = "") -> bool:
        try:
            out = self._hypr_cmd("j/clients")
            clients = json.loads(out)
        except Exception:
            return False

        all_pids = set(pids)
        for p in list(pids):
            curr = p
            while curr > 1:
                try:
                    with open(f"/proc/{curr}/stat", "r") as f:
                        ppid = int(f.read().split()[3])
                        if ppid > 1:
                            all_pids.add(ppid)
                            curr = ppid
                        else:
                            break
                except Exception:
                    break

        target = None
        for client in clients:
            if client.get("pid") in all_pids:
                client_title = client.get("title", "")
                if title and title in client_title:
                    target = client
                    break
                if target is None:
                    target = client

        if target and "address" in target:
            try:
                self._hypr_cmd(f"dispatch focuswindow address:{target['address']}")
                return True
            except Exception:
                pass
        return False

    def is_locked(self) -> bool:
        lockers = {"hyprlock", "swaylock", "waylock"}
        try:
            for entry in os.scandir("/proc"):
                if entry.name.isdigit():
                    try:
                        with open(os.path.join(entry.path, "comm"), "r", encoding="utf-8") as f:
                            if f.read().strip() in lockers:
                                return True
                    except OSError:
                        continue
        except OSError:
            pass
        return False

    def query_hyprland(self):
        now = time.monotonic()

        # 1. Cursor position
        pointer = [0, 0]
        try:
            out = self._hypr_cmd("j/cursorpos")
            pos = json.loads(out)
            pointer = [int(pos.get("x", 0)), int(pos.get("y", 0))]
        except Exception:
            pass

        # 2. Monitors & fullscreen
        covered = []
        monitors_by_ws = {}
        try:
            out = self._hypr_cmd("j/monitors")
            monitors = json.loads(out)
            for m in monitors:
                ws_id = m.get("activeWorkspace", {}).get("id")
                if ws_id is not None:
                    monitors_by_ws[ws_id] = m
        except Exception:
            monitors = []

        # 3. Active window
        active = None
        try:
            out = self._hypr_cmd("j/activewindow")
            win = json.loads(out)
            if win and win.get("mapped", False) and win.get("class") != "Paros":
                at = win.get("at", [0, 0])
                size = win.get("size", [0, 0])
                fs = bool(win.get("fullscreen", 0) or win.get("fullscreenClient", 0))
                active = {
                    "x": at[0],
                    "y": at[1],
                    "width": size[0],
                    "height": size[1],
                    "fullscreen": fs,
                    "pid": win.get("pid", -1),
                }
        except Exception:
            active = None

        # 4. Covered screens (fullscreen detection)
        try:
            out = self._hypr_cmd("j/clients")
            clients = json.loads(out)
            for c in clients:
                if c.get("fullscreen", 0) or c.get("fullscreenClient", 0):
                    ws = c.get("workspace", {}).get("id")
                    if ws in monitors_by_ws:
                        m = monitors_by_ws[ws]
                        rect = [m.get("x", 0), m.get("y", 0), m.get("width", 0), m.get("height", 0)]
                        if rect not in covered:
                            covered.append(rect)
        except Exception:
            pass

        # 5. User activity & idle
        active_pid = active.get("pid", -1) if active else -1
        if pointer != self.last_pointer or active_pid != self.last_active_pid:
            self.last_pointer = pointer
            self.last_active_pid = active_pid
            self.last_activity_at = now

        idle_ms = int(max(0.0, now - self.last_activity_at) * 1000)

        # 6. Screen lock detection
        if self._locked or (now - self._last_lock_check >= 1.5):
            self._last_lock_check = now
            self._locked = self.is_locked()

        return {
            "pointer": pointer,
            "locked": self._locked,
            "mirrored": 0,
            "covered": covered,
            "active": active,
            "idle_ms": idle_ms,
        }

    def tick(self) -> bool:
        if self.parent_pid > 0:
            try:
                os.kill(self.parent_pid, 0)
            except OSError:
                # Parent has exited
                return False

        try:
            state = self.query_hyprland()
            state_str = json.dumps(state)
            now = time.monotonic()
            if state_str != self.last_written or (now - self.last_written_at) >= (HEARTBEAT_MS / 1000.0):
                self.last_written = state_str
                self.last_written_at = now
                tmp_path = self.desktop_json_path + ".tmp"
                with open(tmp_path, "w", encoding="utf-8") as f:
                    f.write(state_str)
                os.replace(tmp_path, self.desktop_json_path)
        except Exception as e:
            sys.stderr.write(f"Paros Hyprland tick error: {e}\n")

        return True

    def cleanup(self):
        try:
            if os.path.exists(self.desktop_json_path):
                os.unlink(self.desktop_json_path)
            tmp_path = self.desktop_json_path + ".tmp"
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)
        except Exception:
            pass


def main():
    parser = argparse.ArgumentParser(description="Paros Hyprland Desktop Helper")
    parser.add_argument("--parent", type=int, default=0, help="Parent PID to monitor")
    args = parser.parse_args()

    helper = ParosHyprlandHelper(parent_pid=args.parent)
    atexit.register(helper.cleanup)

    def sig_handler(signum, frame):
        helper.cleanup()
        sys.exit(0)

    signal.signal(signal.SIGINT, sig_handler)
    signal.signal(signal.SIGTERM, sig_handler)
    if hasattr(signal, "SIGHUP"):
        signal.signal(signal.SIGHUP, sig_handler)

    if gi:
        loop = GLib.MainLoop()

        def on_timeout():
            if not helper.tick():
                loop.quit()
                return False
            return True

        GLib.timeout_add(POLL_MS, on_timeout)
        try:
            loop.run()
        finally:
            helper.cleanup()
    else:
        try:
            while helper.tick():
                time.sleep(POLL_MS / 1000.0)
        finally:
            helper.cleanup()


if __name__ == "__main__":
    main()
