#!/usr/bin/env python3
"""Inspect the installed macOS Bedrock agent/helper pair without mutating it.

This is intentionally a live inspector rather than an installer or restart
wrapper. P15.75 must prove the process boundary and relay state that already
exists on the owner Mac; changing launchd state here would make a failed check
harder to interpret and would hide whether the installed transaction worked.
"""

from __future__ import annotations

import argparse
import getpass
import json
import os
import plistlib
import re
import stat
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path


AGENT_LABEL = "com.ctemple.msc2.agent"
HELPER_LABEL = "com.ctemple.msc2.bedrock-helper"
HELPER_PLIST = Path(f"/Library/LaunchDaemons/{HELPER_LABEL}.plist")
HELPER_ROOT = Path("/Library/Application Support/MSC 2/bedrock-helper")
HELPER_LOG = Path("/var/log/msc2/bedrock-helper.log")
RUNTIME_ROOT = Path("/var/run/msc2")
SOCKET_PATH = RUNTIME_ROOT / "bedrock.sock"
DATA_ROOT = Path.home() / "Library/Application Support/MSC 2"


@dataclass
class Finding:
    label: str
    ok: bool
    detail: str


class Inspector:
    def __init__(self, server_port: int, connection_report: bool) -> None:
        self.server_port = server_port
        self.want_connection_report = connection_report
        self.findings: list[Finding] = []
        self.launchctl_output: dict[str, str] = {}

    def add(self, label: str, ok: bool, detail: str) -> None:
        self.findings.append(Finding(label, ok, detail))
        marker = "PASS" if ok else "FAIL"
        print(f"[{marker}] {label}: {detail}")

    @staticmethod
    def command(*args: str, timeout: float = 5.0) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            list(args),
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )

    def launchctl_print(self, label: str) -> str | None:
        result = self.command("/bin/launchctl", "print", f"system/{label}")
        if result.returncode != 0:
            self.add(
                f"launchd service {label}",
                False,
                (result.stderr or result.stdout).strip() or "service is not loaded",
            )
            return None
        self.launchctl_output[label] = result.stdout
        return result.stdout

    @staticmethod
    def launchd_pid(output: str | None) -> int | None:
        if output is None:
            return None
        match = re.search(r"^\s*pid\s*=\s*(\d+)\s*$", output, re.MULTILINE)
        return int(match.group(1)) if match else None

    @staticmethod
    def process_details(pid: int) -> tuple[str, str, str, str] | None:
        def read_field(format_string: str) -> str | None:
            result = subprocess.run(
                ["/bin/ps", "-ww", "-p", str(pid), "-o", format_string],
                capture_output=True,
                text=True,
                check=False,
            )
            value = result.stdout.strip()
            return value if result.returncode == 0 and value else None

        user = read_field("user=")
        command = read_field("comm=")
        arguments = read_field("args=")
        memory = read_field("%cpu=,rss=")
        if None in (user, command, arguments, memory):
            return None
        return user, command, arguments, memory

    def check_platform(self) -> None:
        system = self.command("/usr/bin/uname", "-s").stdout.strip()
        architecture = self.command("/usr/bin/uname", "-m").stdout.strip()
        self.add(
            "Intel macOS host",
            system == "Darwin" and architecture == "x86_64",
            f"{system or 'unknown'} {architecture or 'unknown'}",
        )

    def check_agent_identity(self, agent_output: str | None) -> None:
        expected_user = getpass.getuser()
        pid = self.launchd_pid(agent_output)
        details = self.process_details(pid) if pid else None
        if details is None:
            self.add("agent process identity", False, "agent PID is not available")
            return
        user, command, arguments, memory = details
        self.add(
            "agent process identity",
            user == expected_user,
            f"pid {pid}, user {user}, command {command}, RSS/CPU {memory}; args {arguments}",
        )

    def check_helper_identity(self, helper_output: str | None) -> int | None:
        pid = self.launchd_pid(helper_output)
        details = self.process_details(pid) if pid else None
        if details is None:
            self.add("helper process identity", False, "helper PID is not available")
            return None
        user, command, arguments, memory = details
        self.add(
            "helper process identity",
            user == "root" and "BedrockSidecar" in arguments,
            f"pid {pid}, user {user}, command {command}, RSS/CPU {memory}; args {arguments}",
        )
        return pid

    def check_helper_resources(self, helper_pid: int | None) -> None:
        if helper_pid is None:
            self.add("helper CPU/RAM observation", False, "helper PID is not available")
            return
        samples: list[tuple[float, int]] = []
        for index in range(2):
            details = self.process_details(helper_pid)
            if details is None:
                self.add("helper CPU/RAM observation", False, "helper process disappeared while sampling")
                return
            resource_fields = details[3].split()
            try:
                samples.append((float(resource_fields[0]), int(resource_fields[1])))
            except (IndexError, ValueError):
                self.add("helper CPU/RAM observation", False, f"unreadable ps resource fields: {details[3]}")
                return
            if index == 0:
                time.sleep(1)
        cpu_values = ", ".join(f"{cpu:.1f}%" for cpu, _ in samples)
        rss_values = ", ".join(f"{rss}K" for _, rss in samples)
        self.add(
            "helper CPU/RAM observation",
            all(rss > 0 for _, rss in samples),
            f"CPU samples={cpu_values}; RSS samples={rss_values}",
        )

    @staticmethod
    def metadata(path: Path) -> os.stat_result | None:
        try:
            return path.lstat()
        except OSError:
            return None

    def check_root_artifact(self, path: Path, *, socket: bool = False) -> None:
        metadata = self.metadata(path)
        if metadata is None:
            self.add(f"artifact {path}", False, "missing")
            return
        mode = stat.S_IMODE(metadata.st_mode)
        if socket:
            expected_uid = os.getuid()
            ok = (
                stat.S_ISSOCK(metadata.st_mode)
                and metadata.st_uid == expected_uid
                and mode == 0o600
            )
            detail = f"type={stat.filemode(metadata.st_mode)}, uid={metadata.st_uid}, mode={mode:04o}"
        else:
            ok = metadata.st_uid == 0 and not (mode & 0o022)
            detail = f"type={stat.filemode(metadata.st_mode)}, uid={metadata.st_uid}, mode={mode:04o}"
        self.add(f"artifact {path}", ok, detail)

    def check_helper_plist(self) -> None:
        try:
            with HELPER_PLIST.open("rb") as handle:
                plist = plistlib.load(handle)
        except (OSError, plistlib.InvalidFileException) as error:
            self.add("helper plist", False, str(error))
            return

        arguments = plist.get("ProgramArguments", [])
        allowed_uid = None
        if "--allowed-uid" in arguments:
            index = arguments.index("--allowed-uid")
            if index + 1 < len(arguments):
                allowed_uid = arguments[index + 1]
        approved_roots = [
            arguments[index + 1]
            for index, argument in enumerate(arguments[:-1])
            if argument == "--approved-root"
        ]
        no_user_name = "UserName" not in plist
        correct_socket = SOCKET_PATH.as_posix() in arguments
        expected_root = DATA_ROOT / "servers"
        correct_root = expected_root.as_posix() in approved_roots
        self.add(
            "helper plist boundary",
            no_user_name
            and correct_socket
            and allowed_uid == str(os.getuid())
            and correct_root,
            f"UserName={'absent' if no_user_name else 'present'}, allowed UID={allowed_uid!r}, socket={SOCKET_PATH}, approved roots={approved_roots!r}, expected root={expected_root}",
        )

    def check_artifacts(self) -> None:
        self.check_helper_plist()
        for path in (
            HELPER_ROOT,
            HELPER_PLIST,
            HELPER_ROOT / "BedrockSidecar",
            HELPER_ROOT / "vmlinuz-kata",
            HELPER_ROOT / "appliance-initramfs.gz",
            RUNTIME_ROOT,
        ):
            self.check_root_artifact(path)
        self.check_root_artifact(SOCKET_PATH, socket=True)

    def read_logs(self) -> list[tuple[Path, str]]:
        paths = [HELPER_LOG, DATA_ROOT / "logs" / "agent.log"]
        server_roots: list[Path] = []
        config_path = DATA_ROOT / "server_config_swift.json"
        try:
            configuration = json.loads(config_path.read_text())
            server_roots = [
                Path(server["server_dir"])
                for server in configuration.get("servers", [])
                if server.get("server_type") == "bedrock" and server.get("server_dir")
            ]
        except (OSError, json.JSONDecodeError, TypeError, KeyError):
            pass
        for server_root in server_roots:
            paths.extend(server_root.glob("**/server.log"))
            paths.extend(server_root.glob("**/logs/*.log"))
        # A missing/legacy config should not turn a Java log containing the
        # same phrase into false Bedrock readiness evidence.
        if not server_roots:
            paths.extend(path for path in DATA_ROOT.glob("**/server.log") if "bedrock" in str(path).lower())
            paths.extend(path for path in DATA_ROOT.glob("**/logs/*.log") if "bedrock" in str(path).lower())
        logs: list[tuple[Path, str]] = []
        seen: set[Path] = set()
        for path in paths:
            if path in seen or not path.is_file():
                continue
            seen.add(path)
            try:
                logs.append((path, path.read_text(errors="replace")))
            except OSError:
                continue
        return logs

    def check_start_evidence(self) -> None:
        logs = self.read_logs()
        started = [path for path, text in logs if re.search(r"server started", text, re.IGNORECASE)]
        rollback = [path for path, text in logs if re.search(r"rollback", text, re.IGNORECASE)]
        self.add(
            "BDS readiness evidence",
            bool(started),
            ", ".join(str(path) for path in started) if started else "no readable log contains 'Server started'",
        )
        self.add(
            "start operation rollback evidence",
            bool(started) and not rollback,
            ", ".join(str(path) for path in rollback)
            if rollback
            else ("cannot evaluate before BDS readiness is recorded" if not started else "no readable log contains 'rollback'"),
        )

    def check_udp_ports(self) -> None:
        result = self.command("/usr/sbin/netstat", "-anv", "-p", "udp")
        listening_ports = {
            int(match.group(1))
            for match in re.finditer(r"^udp\S*\s+.*?\*\.(\d+)\s+\*\.\*", result.stdout, re.MULTILINE)
        }
        bound = self.server_port in listening_ports
        self.add(
            f"UDP RakNet relay {self.server_port}",
            bound,
            "bound" if bound else "missing",
        )

    def check_process_cleanup(self, helper_pid: int | None) -> None:
        result = self.command("/bin/ps", "-axo", "pid=,ppid=,user=,comm=,args=")
        rows: list[str] = []
        orphan_sidecars: list[str] = []
        for line in result.stdout.splitlines():
            if not re.search(r"BedrockSidecar|bedrock_server|vmlinuz-kata", line):
                continue
            rows.append(line.strip())
            fields = line.split(None, 4)
            if fields and "BedrockSidecar" in line and helper_pid is not None:
                try:
                    if int(fields[0]) != helper_pid:
                        orphan_sidecars.append(line.strip())
                except ValueError:
                    orphan_sidecars.append(line.strip())
            elif "BedrockSidecar" in line and helper_pid is None:
                orphan_sidecars.append(line.strip())
        self.add(
            "no orphan Bedrock VM/sidecar process",
            not orphan_sidecars,
            "active managed process: " + "; ".join(rows) if rows and not orphan_sidecars else ("; ".join(orphan_sidecars) or "none"),
        )

    def connection_report(self) -> None:
        if not self.want_connection_report:
            return
        # The helper is root-owned, so lsof run by the installing user cannot
        # see its sockets. netstat still exposes the listener boundary and is
        # sufficient for this report; process ownership is checked separately.
        tcp_result = self.command("/usr/sbin/netstat", "-anv", "-p", "tcp")
        udp_result = self.command("/usr/sbin/netstat", "-anv", "-p", "udp")
        relevant = [
            line.strip()
            for line in (tcp_result.stdout + udp_result.stdout).splitlines()
            if any(token in line for token in ("BedrockSidecar", "bedrock_server", "19001", "19002", "19034"))
        ]
        self.add(
            "connection report",
            bool(relevant),
            "; ".join(relevant) if relevant else "no relevant listeners or connections",
        )

    def run(self) -> int:
        self.check_platform()
        agent_output = self.launchctl_print(AGENT_LABEL)
        helper_output = self.launchctl_print(HELPER_LABEL)
        self.check_agent_identity(agent_output)
        helper_pid = self.check_helper_identity(helper_output)
        self.check_helper_resources(helper_pid)
        self.check_artifacts()
        self.check_start_evidence()
        self.check_udp_ports()
        self.check_process_cleanup(helper_pid)
        self.connection_report()

        failures = sum(not finding.ok for finding in self.findings)
        print(f"\n{len(self.findings) - failures} passed, {failures} failed")
        return 0 if failures == 0 else 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--live", action="store_true", help="inspect the installed local pair")
    parser.add_argument("--server-port", type=int, default=19001)
    parser.add_argument("--connection-report", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.live:
        print("refusing to run without --live; this command is an installed-pair acceptance check", file=sys.stderr)
        return 2
    return Inspector(args.server_port, args.connection_report).run()


if __name__ == "__main__":
    raise SystemExit(main())
