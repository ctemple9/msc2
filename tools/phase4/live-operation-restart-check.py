#!/usr/bin/env python3
"""P4.17 live check: lifecycle operation restart reconciliation.

The script owns its temporary agent process. It uses the narrow
MSC2_TEST_BOOTSTRAP_TOKEN hook because the real CLI token helper lands in
P4.18, one step after this check.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


TOKEN = "msc2_phase4live_secret"


def request(base_url: str, method: str, path: str, body: dict | None = None) -> tuple[int, dict]:
    data = None
    headers = {"Authorization": f"Bearer {TOKEN}"}
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(
        base_url.rstrip("/") + path,
        data=data,
        headers=headers,
        method=method,
    )
    with urllib.request.urlopen(req, timeout=5) as resp:
        return resp.status, json.loads(resp.read())


def wait_for_health(base_url: str, proc: subprocess.Popen) -> None:
    deadline = time.time() + 45
    while time.time() < deadline:
        if proc.poll() is not None:
            stderr = proc.stderr.read() if proc.stderr else ""
            raise RuntimeError(f"agent exited early with code {proc.returncode}: {stderr[-2000:]}")
        try:
            with urllib.request.urlopen(base_url.rstrip("/") + "/v1/health", timeout=1) as resp:
                if resp.status == 200:
                    return
        except (urllib.error.URLError, TimeoutError):
            time.sleep(0.25)
    raise RuntimeError("agent did not become healthy")


def start_agent(base_url: str, journal_dir: Path) -> subprocess.Popen:
    parsed = urllib.parse.urlparse(base_url)
    bind = f"{parsed.hostname}:{parsed.port or 80}"
    env = os.environ.copy()
    env["MSC2_TEST_BOOTSTRAP_TOKEN"] = TOKEN
    env["MSC2_OPERATION_JOURNAL_DIR"] = str(journal_dir)
    proc = subprocess.Popen(
        ["cargo", "run", "-p", "msc-agent", "--", "serve", "--bind", bind],
        cwd=Path(__file__).resolve().parents[2],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    wait_for_health(base_url, proc)
    return proc


def stop_agent(proc: subprocess.Popen) -> None:
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=10)


def build_fake_paper(server_dir: Path) -> None:
    if shutil.which("javac") is None or shutil.which("jar") is None:
        raise RuntimeError("javac and jar are required for the live Paper lifecycle check")
    source = server_dir / "FakePaper.java"
    source.write_text(
        """
import java.io.BufferedReader;
import java.io.InputStreamReader;

public class FakePaper {
    public static void main(String[] args) throws Exception {
        System.out.println("Booting fake Paper");
        System.out.flush();
        Thread.sleep(2500);
        System.out.println("Done (0.001s)! For help, type \\"help\\"");
        System.out.flush();
        BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        String line;
        while ((line = reader.readLine()) != null) {
            if (line.equals("stop")) {
                System.out.println("Stopping fake Paper");
                return;
            }
        }
    }
}
""".strip()
    )
    subprocess.run(["javac", str(source)], check=True, cwd=server_dir)
    manifest = server_dir / "manifest.txt"
    manifest.write_text("Main-Class: FakePaper\n")
    subprocess.run(
        ["jar", "cfm", "paper.jar", "manifest.txt", "FakePaper.class"],
        check=True,
        cwd=server_dir,
    )
    (server_dir / "eula.txt").write_text("eula=true\n")
    (server_dir / "server.properties").write_text(
        "server-port=25565\nmax-players=20\nlevel-name=world\n"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", required=True)
    args = parser.parse_args()

    with tempfile.TemporaryDirectory(prefix="msc2-p4-operation-") as tmp:
        tmp_path = Path(tmp)
        journal_dir = tmp_path / "journal"
        journal_dir.mkdir()
        server_dir = tmp_path / "paper"
        server_dir.mkdir()
        build_fake_paper(server_dir)

        first = start_agent(args.base_url, journal_dir)
        try:
            _, imported = request(
                args.base_url,
                "POST",
                "/v1/servers/import",
                {
                    "action": "importExisting",
                    "sourcePath": str(server_dir),
                    "displayName": "Operation Paper",
                    "serverType": "java",
                    "importKind": "paper",
                },
            )
            server_id = imported["serverId"]
            request(args.base_url, "POST", "/v1/active-server", {"serverId": server_id})
            _, started = request(args.base_url, "POST", "/v1/start")
            operation_id = started.get("operationId")
            if not operation_id:
                raise RuntimeError(f"start response did not include operationId: {started}")
            _, running = request(args.base_url, "GET", f"/v1/operations/{operation_id}")
            if running.get("state") != "running":
                raise RuntimeError(f"expected running operation before restart, got {running}")
        finally:
            stop_agent(first)

        second = start_agent(args.base_url, journal_dir)
        try:
            _, reconciled = request(args.base_url, "GET", f"/v1/operations/{operation_id}")
            if reconciled.get("state") != "failed":
                raise RuntimeError(f"expected failed reconciled operation, got {reconciled}")
            error = reconciled.get("error") or {}
            if error.get("code") != "operation_interrupted":
                raise RuntimeError(f"expected operation_interrupted error, got {reconciled}")
        finally:
            stop_agent(second)

    print("ok live-operation-restart")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"FAIL live-operation-restart: {exc}", file=sys.stderr)
        raise SystemExit(1)
