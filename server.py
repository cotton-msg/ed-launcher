#!/usr/bin/env python3
"""
Grand Eden Server
=================
Хостинг зипок + API инвентаря игроков.
Запуск: python server.py [--port 25566] [--dir ./files]

Структура файлов:
  files/
    java.zip
    version.zip

API:
  POST   /api/inventory          — принять отчёт от лаунчера
  GET    /api/inventory           — список всех игроков
  GET    /api/inventory/<nick>    — файлы конкретного игрока
  DELETE /api/inventory/<nick>    — удалить игрока
  GET    /                       — панель управления (panel.html)
"""

import argparse
import json
import os
import sys
import time
import hashlib
import hmac
from http.server import HTTPServer, SimpleHTTPRequestHandler
from urllib.parse import urlparse, unquote
from datetime import datetime, timezone

DATA_FILE = "inventory.json"
SECRET_KEY = ""  # Если хочешь защиту — поставь пароль. Пусто = без авторизации


def load_inventory(data_file):
    if os.path.exists(data_file):
        with open(data_file, "r", encoding="utf-8") as f:
            return json.load(f)
    return {"players": {}}


def save_inventory(data_file, data):
    with open(data_file, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


class GrandEdenHandler(SimpleHTTPRequestHandler):
    server_version = "GrandEden/1.0"

    def log_message(self, fmt, *args):
        ts = datetime.now().strftime("%H:%M:%S")
        sys.stderr.write(f"[{ts}] {args[0]}\n")

    def _send_json(self, code, obj):
        body = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _read_body(self):
        length = int(self.headers.get("Content-Length", 0))
        return self.rfile.read(length)

    def _extract_nick(self, path):
        parts = path.strip("/").split("/")
        if len(parts) >= 3:
            return unquote(parts[2])
        return None

    def do_OPTIONS(self):
        self.send_response(204)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        self.end_headers()

    def do_GET(self):
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/")

        if path == "" or path == "/panel" or path == "/panel.html":
            panel_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "panel.html")
            if os.path.exists(panel_path):
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.end_headers()
                with open(panel_path, "rb") as f:
                    self.wfile.write(f.read())
            else:
                self._send_json(404, {"error": "panel.html not found"})
            return

        if path == "/api/inventory":
            data = load_inventory(DATA_FILE)
            players = []
            for nick, info in data["players"].items():
                players.append({
                    "nickname": nick,
                    "files": info.get("files", []),
                    "total_size": info.get("total_size", 0),
                    "file_count": info.get("file_count", 0),
                    "reported_at": info.get("reported_at", ""),
                })
            players.sort(key=lambda p: p.get("reported_at", ""), reverse=True)
            self._send_json(200, players)
            return

        if path.startswith("/api/inventory/"):
            nick = self._extract_nick(path)
            if not nick:
                self._send_json(400, {"error": "missing nickname"})
                return
            data = load_inventory(DATA_FILE)
            if nick in data["players"]:
                info = data["players"][nick]
                info["nickname"] = nick
                self._send_json(200, info)
            else:
                self._send_json(404, {"error": f"player '{nick}' not found"})
            return

        self._send_json(404, {"error": "not found"})

    def do_POST(self):
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/")

        if path == "/api/inventory":
            try:
                body = json.loads(self._read_body())
            except Exception:
                self._send_json(400, {"error": "invalid JSON"})
                return

            nickname = body.get("nickname", "").strip()
            if not nickname:
                self._send_json(400, {"error": "nickname required"})
                return

            data = load_inventory(DATA_FILE)
            data["players"][nickname] = {
                "files": body.get("files", []),
                "total_size": body.get("total_size", 0),
                "file_count": body.get("file_count", 0),
                "reported_at": datetime.now(timezone.utc).isoformat(),
            }
            save_inventory(DATA_FILE, data)
            self._send_json(200, {"ok": True, "nickname": nickname})
            return

        self._send_json(404, {"error": "not found"})

    def do_DELETE(self):
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/")

        if path.startswith("/api/inventory/"):
            nick = self._extract_nick(path)
            if not nick:
                self._send_json(400, {"error": "missing nickname"})
                return
            data = load_inventory(DATA_FILE)
            if nick in data["players"]:
                del data["players"][nick]
                save_inventory(DATA_FILE, data)
                self._send_json(200, {"ok": True, "deleted": nick})
            else:
                self._send_json(404, {"error": f"player '{nick}' not found"})
            return

        self._send_json(404, {"error": "not found"})


def main():
    parser = argparse.ArgumentParser(description="Grand Eden Server")
    parser.add_argument("--port", type=int, default=25566, help="Port (default: 25566)")
    parser.add_argument("--dir", type=str, default="files", help="Directory with java.zip/version.zip")
    parser.add_argument("--host", type=str, default="0.0.0.0", help="Bind address")
    args = parser.parse_args()

    file_dir = os.path.abspath(args.dir)
    os.makedirs(file_dir, exist_ok=True)

    os.chdir(file_dir)

    # If panel.html is not in cwd, set directory to find it
    original_dir = os.path.dirname(os.path.abspath(__file__))
    panel_src = os.path.join(original_dir, "panel.html")
    panel_dst = os.path.join(file_dir, "panel.html")
    if os.path.exists(panel_src) and not os.path.exists(panel_dst):
        import shutil
        shutil.copy2(panel_src, panel_dst)

    print(f"╔══════════════════════════════════════╗")
    print(f"║     Grand Eden Server v1.0           ║")
    print(f"╠══════════════════════════════════════╣")
    print(f"║  Файлы:   {file_dir}")
    print(f"║  Порт:    {args.port}")
    print(f"║  Панель:  http://localhost:{args.port}/")
    print(f"║  API:     http://localhost:{args.port}/api/inventory")
    print(f"╚══════════════════════════════════════╝")
    print(f"\nОжидаю подключений...\n")

    server = HTTPServer((args.host, args.port), GrandEdenHandler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nОстановлен.")
        server.server_close()


if __name__ == "__main__":
    main()
