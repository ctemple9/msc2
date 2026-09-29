import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";

function version(command, args) {
  return execFileSync(command, args, { encoding: "utf8" }).trim();
}

function npmVersion() {
  if (process.platform === "win32") {
    return version("cmd.exe", ["/d", "/s", "/c", "npm --version"]);
  }
  return version("npm", ["--version"]);
}

const [outputPath] = process.argv.slice(2);
if (!outputPath) {
  throw new Error("usage: record-builder-environment.mjs <output-path>");
}

const record = {
  schemaVersion: 1,
  sourceCommit: process.env.GITHUB_SHA,
  platform: process.env.RELEASE_PLATFORM,
  runner: {
    imageOS: process.env.ImageOS ?? "unknown",
    imageVersion: process.env.ImageVersion ?? "unknown",
  },
  toolchain: {
    rustc: version("rustc", ["--version"]),
    cargo: version("cargo", ["--version"]),
    node: version(process.execPath, ["--version"]),
    npm: npmVersion(),
    cargoNextest: version("cargo", ["nextest", "--version"]).split(/\s+/)[1],
  },
};

writeFileSync(outputPath, `${JSON.stringify(record, null, 2)}\n`, "utf8");
