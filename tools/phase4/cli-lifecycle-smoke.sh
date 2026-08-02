#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/msc2-cli-smoke.XXXXXX")"
PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
BASE_URL="http://127.0.0.1:${PORT}"
TOKEN="msc2_cli_smoke_bootstrap_secret"
MSC_BIN="${ROOT}/target/debug/msc"
AGENT_PID=""

cleanup() {
  if [[ -n "${AGENT_PID}" ]] && kill -0 "${AGENT_PID}" 2>/dev/null; then
    kill "${AGENT_PID}" 2>/dev/null || true
    wait "${AGENT_PID}" 2>/dev/null || true
  fi
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required tool: $1" >&2
    exit 1
  fi
}

require_tool cargo
require_tool python3
require_tool javac
require_tool jar

mkdir -p "${TMP_DIR}/paper"

cat > "${TMP_DIR}/paper/FakePaper.java" <<'EOF'
import java.io.BufferedReader;
import java.io.InputStreamReader;

public class FakePaper {
    public static void main(String[] args) throws Exception {
        System.out.println("Booting fake Paper");
        System.out.flush();
        Thread.sleep(1500);
        System.out.println("Done (0.001s)! For help, type \"help\"");
        System.out.flush();

        BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        String line;
        while ((line = reader.readLine()) != null) {
            System.out.println("COMMAND:" + line);
            System.out.flush();
            if (line.equals("stop")) {
                System.out.println("Stopping fake Paper");
                System.out.flush();
                return;
            }
        }
    }
}
EOF

cat > "${TMP_DIR}/paper/server.properties" <<'EOF'
server-port=25565
max-players=20
level-name=world
EOF

cat > "${TMP_DIR}/paper/eula.txt" <<'EOF'
eula=true
EOF

(
  cd "${TMP_DIR}/paper"
  javac FakePaper.java
  printf 'Main-Class: FakePaper\n' > manifest.txt
  jar cfm paper.jar manifest.txt FakePaper.class >/dev/null
)

(
  cd "${ROOT}"
  cargo build -p msc-agent >/dev/null
)

export MSC2_TEST_BOOTSTRAP_TOKEN="${TOKEN}"
export MSC2_OPERATION_JOURNAL_DIR="${TMP_DIR}/journal"
mkdir -p "${MSC2_OPERATION_JOURNAL_DIR}"

"${MSC_BIN}" serve --bind "127.0.0.1:${PORT}" >"${TMP_DIR}/agent.log" 2>&1 &
AGENT_PID="$!"

python3 - "${BASE_URL}" <<'PY'
import sys
import time
import urllib.error
import urllib.request

base_url = sys.argv[1]
deadline = time.time() + 45
while time.time() < deadline:
    try:
        with urllib.request.urlopen(base_url + "/v1/health", timeout=1) as resp:
            if resp.status == 200:
                raise SystemExit(0)
    except (urllib.error.URLError, TimeoutError):
        time.sleep(0.25)
raise SystemExit("agent did not become healthy")
PY

TOKEN_FROM_CLI="$("${MSC_BIN}" token print --test)"

"${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" server import "${TMP_DIR}/paper" --name "Smoke Paper" >/dev/null
"${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" server start "Smoke Paper" >/dev/null

python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys
import time

msc, base_url, token = sys.argv[1:4]
deadline = time.time() + 45
while time.time() < deadline:
    output = subprocess.check_output(
        [msc, "--base-url", base_url, "--token", token, "--json", "status"],
        text=True,
    )
    if json.loads(output)["running"]:
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("server never reached running state")
PY

python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys
import time

msc, base_url, token = sys.argv[1:4]
deadline = time.time() + 45
while time.time() < deadline:
    output = subprocess.check_output(
        [
            msc,
            "--base-url",
            base_url,
            "--token",
            token,
            "--json",
            "console",
            "tail",
            "--server",
            "Smoke Paper",
            "--lines",
            "50",
        ],
        text=True,
    )
    lines = json.loads(output)
    if any('Done (0.001s)! For help, type "help"' in line["text"] for line in lines):
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("server never emitted the ready line")
PY

"${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" command --server "Smoke Paper" "say smoke test" >/dev/null

python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys
import time

msc, base_url, token = sys.argv[1:4]
deadline = time.time() + 20
while time.time() < deadline:
    output = subprocess.check_output(
        [
            msc,
            "--base-url",
            base_url,
            "--token",
            token,
            "--json",
            "console",
            "tail",
            "--server",
            "Smoke Paper",
            "--lines",
            "20",
        ],
        text=True,
    )
    lines = json.loads(output)
    if any("COMMAND:say smoke test" in line["text"] for line in lines):
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("console tail never observed the sent command")
PY

"${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" server restart "Smoke Paper" >/dev/null

python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys
import time

msc, base_url, token = sys.argv[1:4]
deadline = time.time() + 45
while time.time() < deadline:
    console = subprocess.check_output(
        [
            msc,
            "--base-url",
            base_url,
            "--token",
            token,
            "--json",
            "console",
            "tail",
            "--server",
            "Smoke Paper",
            "--lines",
            "50",
        ],
        text=True,
    )
    output = subprocess.check_output(
        [msc, "--base-url", base_url, "--token", token, "--json", "status"],
        text=True,
    )
    lines = json.loads(console)
    if json.loads(output)["running"] and any(
        'Done (0.001s)! For help, type "help"' in line["text"] for line in lines
    ):
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("server never returned to ready after restart")
PY

"${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" server stop "Smoke Paper" >/dev/null

python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys
import time

msc, base_url, token = sys.argv[1:4]
deadline = time.time() + 30
while time.time() < deadline:
    output = subprocess.check_output(
        [msc, "--base-url", base_url, "--token", token, "--json", "status"],
        text=True,
    )
    if not json.loads(output)["running"]:
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("server never stopped")
PY

echo "cli lifecycle smoke passed"
