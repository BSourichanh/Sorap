#!/usr/bin/env python3
"""Reads session summaries and metadata from Antigravity Desktop SQLite DB."""
import json
import os
import sqlite3
import sys
import urllib.parse


def read_antigravity_db(db_path: str = "") -> dict:
    if not db_path:
        db_path = os.path.expanduser("~/.gemini/antigravity/conversation_summaries.db")

    if not os.path.isfile(db_path):
        return {}

    try:
        conn = sqlite3.connect(f"file:{os.path.abspath(db_path)}?mode=ro", uri=True, timeout=1.0)
        cursor = conn.cursor()
        cursor.execute(
            """
            SELECT conversation_id, title, status, workspace_uris, parent_conversation_id, not_fully_idle
            FROM conversation_summaries
            WHERE killed = 0
            """
        )
        sessions = {}
        for conv_id, title, status, ws_uris, parent_id, not_idle in cursor.fetchall():
            cwd = ""
            if ws_uris:
                try:
                    uris = json.loads(ws_uris)
                    if isinstance(uris, list) and uris:
                        first = uris[0]
                        if first.startswith("file://"):
                            first = first[7:]
                        cwd = urllib.parse.unquote(first)
                except Exception:
                    pass

            sessions[conv_id] = {
                "title": title or "",
                "status": status or "",
                "cwd": cwd,
                "parent_id": parent_id or "",
                "not_idle": bool(not_idle),
            }
        conn.close()
        return sessions
    except Exception:
        return {}


def main():
    db_path = ""
    if "--db" in sys.argv:
        idx = sys.argv.index("--db")
        if idx + 1 < len(sys.argv):
            db_path = sys.argv[idx + 1]

    data = read_antigravity_db(db_path)
    sys.stdout.write(json.dumps(data))
    sys.stdout.flush()


if __name__ == "__main__":
    main()
