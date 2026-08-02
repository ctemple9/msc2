#!/usr/bin/env python3
"""P4.27 live Paper lifecycle conformance check.

Drives the Phase 4 non-service vertical slice against a real Paper server
directory using only the public CLI and HTTP/WebSocket API surface.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any


TOKEN = "msc2_phase4_live_paper_secret"
READY_NEEDLE = 'For help, type "help"'
COMMAND_NEEDLE = "phase4 live lifecycle check"


def fail(message: str) -> RuntimeError:
    return RuntimeError(message)


def request(
    base_url: str,
    method: str,
    path: str,
    token: str,
    body: dict[str, Any] | None = None,
) -> tuple[int, Any]:
    data = None
    headers = {"Authorization": f"Bearer {token}"}
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(
        base_url.rstrip("/") + path,
        data=data,
        headers=headers,
        method=method,
    )
    with urllib.request.urlopen(req, timeout=5) as response:
        payload = response.read()
        if not payload:
            return response.status, None
        return response.status, json.loads(payload)


def wait_for_health(base_url: str, log_path: Path) -> None:
    deadline = time.time() + 45
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(base_url.rstrip("/") + "/v1/health", timeout=1) as resp:
                if resp.status == 200:
                    return
        except (urllib.error.URLError, TimeoutError):
            time.sleep(0.25)
    raise fail(f"agent did not become healthy; recent log:\n{tail_text(log_path)}")


def tail_text(path: Path, lines: int = 40) -> str:
    if not path.exists():
        return "(log file missing)"
    text = path.read_text(errors="replace")
    return "\n".join(text.splitlines()[-lines:])


def run_cli(msc_bin: Path, base_url: str, args: list[str], token: str) -> str:
    env = os.environ.copy()
    env["MSC2_TEST_BOOTSTRAP_TOKEN"] = token
    env["MSC2_CLI_TOKEN"] = token
    completed = subprocess.run(
        [str(msc_bin), "--base-url", base_url, "--token", token, *args],
        check=True,
        text=True,
        capture_output=True,
        env=env,
    )
    return completed.stdout


def run_cli_json(msc_bin: Path, base_url: str, args: list[str], token: str) -> Any:
    output = run_cli(msc_bin, base_url, ["--json", *args], token)
    return json.loads(output)


def start_agent(
    repo_root: Path,
    msc_bin: Path,
    base_url: str,
    journal_dir: Path,
    log_path: Path,
) -> subprocess.Popen[str]:
    parsed = urllib.parse.urlparse(base_url)
    if parsed.scheme != "http":
        raise fail(f"only http base URLs are supported, got {base_url}")
    if not parsed.hostname or not parsed.port:
        raise fail(f"base URL must include host and port, got {base_url}")
    bind = f"{parsed.hostname}:{parsed.port}"
    env = os.environ.copy()
    env["MSC2_TEST_BOOTSTRAP_TOKEN"] = TOKEN
    env["MSC2_OPERATION_JOURNAL_DIR"] = str(journal_dir)
    log_handle = log_path.open("w")
    try:
        proc = subprocess.Popen(
            [str(msc_bin), "serve", "--bind", bind],
            cwd=repo_root,
            env=env,
            stdout=log_handle,
            stderr=subprocess.STDOUT,
            text=True,
        )
    except Exception:
        log_handle.close()
        raise
    wait_for_health(base_url, log_path)
    return proc


def stop_agent(proc: subprocess.Popen[str]) -> None:
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=10)


def wait_for_status(
    base_url: str,
    token: str,
    *,
    running: bool,
    active_server_id: str,
    require_pid: bool,
) -> dict[str, Any]:
    deadline = time.time() + 60
    while time.time() < deadline:
        _, payload = request(base_url, "GET", "/v1/status", token)
        if (
            payload.get("running") is running
            and payload.get("activeServerId") == active_server_id
            and (not require_pid or payload.get("pid"))
        ):
            return payload
        time.sleep(0.25)
    raise fail(f"status never reached running={running}: {payload}")


def wait_for_performance(base_url: str, token: str) -> dict[str, Any]:
    deadline = time.time() + 20
    while time.time() < deadline:
        _, payload = request(base_url, "GET", "/v1/performance", token)
        if payload.get("ts") and payload.get("serverType") in {"java", "paper"}:
            return payload
        time.sleep(0.25)
    raise fail(f"performance route never returned a Phase 4 server snapshot: {payload}")


def wait_for_console_tail(
    base_url: str,
    token: str,
    needle: str,
    *,
    min_ts: int = 0,
    timeout: float = 45,
) -> dict[str, Any]:
    deadline = time.time() + timeout
    while time.time() < deadline:
        _, payload = request(base_url, "GET", "/v1/console/tail?n=200", token)
        for line in payload:
            if needle in line.get("text", "") and int(line.get("ts", "0")) >= min_ts:
                return line
        time.sleep(0.25)
    raise fail(f"console tail never observed {needle!r}")


class WebSocketReader:
    def __init__(self, base_url: str, token: str, path: str) -> None:
        parsed = urllib.parse.urlparse(base_url)
        if parsed.scheme != "http":
            raise fail(f"websocket helper only supports http base URLs, got {base_url}")
        self.host = parsed.hostname or "127.0.0.1"
        self.port = parsed.port or 80
        self.path = f"{parsed.path.rstrip('/')}{path}"
        self.token = token
        self.sock: socket.socket | None = None

    def __enter__(self) -> "WebSocketReader":
        key = base64.b64encode(os.urandom(16)).decode()
        request_text = (
            f"GET {self.path} HTTP/1.1\r\n"
            f"Host: {self.host}:{self.port}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n"
            f"Authorization: Bearer {self.token}\r\n"
            "\r\n"
        )
        sock = socket.create_connection((self.host, self.port), timeout=5)
        sock.sendall(request_text.encode())
        response = self._read_http_headers(sock)
        status_line = response.split(b"\r\n", 1)[0].decode(errors="replace")
        if " 101 " not in status_line:
            sock.close()
            raise fail(f"websocket upgrade failed: {status_line}")
        self.sock = sock
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        if self.sock is not None:
            self.sock.close()
            self.sock = None

    def _read_http_headers(self, sock: socket.socket) -> bytes:
        data = b""
        while b"\r\n\r\n" not in data:
            chunk = sock.recv(4096)
            if not chunk:
                raise fail("websocket upgrade response ended before headers completed")
            data += chunk
        return data

    def read_json_message(self, timeout: float) -> dict[str, Any]:
        if self.sock is None:
            raise fail("websocket is not connected")
        self.sock.settimeout(timeout)
        first = self._recv_exact(2)
        first_byte, second_byte = first[0], first[1]
        opcode = first_byte & 0x0F
        masked = (second_byte & 0x80) != 0
        length = second_byte & 0x7F
        if masked:
            raise fail("server unexpectedly masked websocket frame")
        if length == 126:
            length = int.from_bytes(self._recv_exact(2), "big")
        elif length == 127:
            length = int.from_bytes(self._recv_exact(8), "big")
        payload = self._recv_exact(length)
        if opcode == 0x8:
            raise fail("websocket closed before expected message arrived")
        if opcode != 0x1:
            raise fail(f"unexpected websocket opcode {opcode}")
        return json.loads(payload.decode())

    def _recv_exact(self, count: int) -> bytes:
        assert self.sock is not None
        data = b""
        while len(data) < count:
            chunk = self.sock.recv(count - len(data))
            if not chunk:
                raise fail("websocket closed mid-frame")
            data += chunk
        return data


def wait_for_websocket_line(
    reader: WebSocketReader,
    needle: str,
    *,
    min_ts: int,
    timeout: float,
) -> dict[str, Any]:
    deadline = time.time() + timeout
    while time.time() < deadline:
        remaining = max(0.1, deadline - time.time())
        line = reader.read_json_message(remaining)
        if needle in line.get("text", "") and int(line.get("ts", "0")) >= min_ts:
            return line
    raise fail(f"websocket never observed {needle!r}")


def build_agent(repo_root: Path) -> Path:
    subprocess.run(["cargo", "build", "-p", "msc-agent"], cwd=repo_root, check=True)
    msc_bin = repo_root / "target" / "debug" / "msc"
    if not msc_bin.exists():
        raise fail(f"expected built CLI at {msc_bin}")
    return msc_bin


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--server-dir", required=True)
    parser.add_argument("--base-url", required=True)
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[2]
    server_dir = Path(args.server_dir).expanduser().resolve()
    if not server_dir.is_dir():
        raise fail(f"server directory does not exist: {server_dir}")
    if not (server_dir / "paper.jar").is_file():
        raise fail(f"server directory is missing paper.jar: {server_dir}")
    if not (server_dir / "server.properties").is_file():
        raise fail(f"server directory is missing server.properties: {server_dir}")

    msc_bin = build_agent(repo_root)

    with tempfile.TemporaryDirectory(prefix="msc2-phase4-live-paper-") as temp_dir:
        temp_root = Path(temp_dir)
        journal_dir = temp_root / "journal"
        journal_dir.mkdir()
        log_path = temp_root / "agent.log"
        server_name = f"Phase4 Live Paper {int(time.time())}"

        agent = start_agent(repo_root, msc_bin, args.base_url, journal_dir, log_path)
        try:
            token = run_cli(msc_bin, args.base_url, ["token", "print", "--test"], TOKEN).strip()
            if token != TOKEN:
                raise fail("CLI test-token resolution did not return the bootstrap token")

            _, detection = request(
                args.base_url,
                "POST",
                "/v1/servers/import",
                token,
                {"action": "scan", "sourcePath": str(server_dir)},
            )
            if detection.get("serverType") != "java" or detection.get("javaFlavor") != "paper":
                raise fail(f"scan did not detect a Paper Java server: {detection}")

            imported = run_cli_json(
                msc_bin,
                args.base_url,
                ["server", "import", str(server_dir), "--name", server_name],
                token,
            )
            server_id = imported.get("serverId")
            if not server_id:
                raise fail(f"import did not return serverId: {imported}")

            _, servers = request(args.base_url, "GET", "/v1/servers", token)
            server = next((item for item in servers if item.get("id") == server_id), None)
            if server is None:
                raise fail(f"imported server {server_id} not present in /v1/servers")
            if server.get("directory") != str(server_dir):
                raise fail(f"server directory mismatch after import: {server}")
            if server.get("serverType") != "java" or server.get("javaFlavor") != "paper":
                raise fail(f"imported server type mismatch: {server}")

            _, selected = request(
                args.base_url,
                "POST",
                "/v1/active-server",
                token,
                {"serverId": server_id},
            )
            if selected.get("activeServerId") != server_id:
                raise fail(f"active-server selection failed: {selected}")

            start_at_ms = int(time.time() * 1000)
            run_cli(msc_bin, args.base_url, ["server", "start", server_name], token)
            wait_for_status(
                args.base_url,
                token,
                running=True,
                active_server_id=server_id,
                require_pid=True,
            )
            wait_for_performance(args.base_url, token)
            wait_for_console_tail(
                args.base_url,
                token,
                READY_NEEDLE,
                min_ts=start_at_ms,
            )

            with WebSocketReader(args.base_url, token, "/v1/console/stream") as websocket:
                command_at_ms = int(time.time() * 1000)
                run_cli(
                    msc_bin,
                    args.base_url,
                    ["command", "--server", server_name, f"say {COMMAND_NEEDLE}"],
                    token,
                )
                wait_for_console_tail(
                    args.base_url,
                    token,
                    COMMAND_NEEDLE,
                    min_ts=command_at_ms,
                    timeout=20,
                )
                wait_for_websocket_line(
                    websocket,
                    COMMAND_NEEDLE,
                    min_ts=command_at_ms,
                    timeout=20,
                )

            run_cli(msc_bin, args.base_url, ["server", "stop", server_name], token)
            wait_for_status(
                args.base_url,
                token,
                running=False,
                active_server_id=server_id,
                require_pid=False,
            )

            with WebSocketReader(args.base_url, token, "/v1/console/stream") as websocket:
                restart_at_ms = int(time.time() * 1000)
                run_cli(msc_bin, args.base_url, ["server", "restart", server_name], token)
                wait_for_status(
                    args.base_url,
                    token,
                    running=True,
                    active_server_id=server_id,
                    require_pid=True,
                )
                wait_for_websocket_line(
                    websocket,
                    READY_NEEDLE,
                    min_ts=restart_at_ms,
                    timeout=45,
                )

            run_cli(msc_bin, args.base_url, ["server", "stop", server_name], token)
            wait_for_status(
                args.base_url,
                token,
                running=False,
                active_server_id=server_id,
                require_pid=False,
            )
        finally:
            stop_agent(agent)

    print("live Paper lifecycle check passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as exc:
        if exc.stderr:
            print(exc.stderr, file=sys.stderr, end="")
        raise SystemExit(exc.returncode)
    except Exception as exc:
        print(f"FAIL live-paper-lifecycle: {exc}", file=sys.stderr)
        raise SystemExit(1)
