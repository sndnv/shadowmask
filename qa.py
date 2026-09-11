#!/usr/bin/env python3
import shutil
import subprocess
import sys
from pathlib import Path

FAIL_UNDER_LINES = "99.6"
IGNORE_COVERAGE = r"crates/server/src/main\.rs$"

STEPS = [
    ("fmt", ["cargo", "fmt", "--all", "--check"]),
    ("clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"]),
    ("build", ["cargo", "build", "--workspace", "--locked"]),
    ("deny", ["cargo", "deny", "check"]),
    ("coverage", ["cargo", "llvm-cov", "nextest", "--workspace", "--locked", "--ignore-filename-regex", IGNORE_COVERAGE, "--fail-under-lines", FAIL_UNDER_LINES]),
]

DEPS_CMD = ["cargo", "update", "--dry-run"]

STEP_TOOLS = {
    "deny": ["cargo-deny"],
    "coverage": ["cargo-nextest", "cargo-llvm-cov", "ffmpeg", "ffprobe"],
}

INSTALL_HINTS = {
    "ffmpeg": "ffmpeg + ffprobe: macOS `brew install ffmpeg`, Ubuntu `sudo apt-get install -y ffmpeg`",
    "ffprobe": "ffmpeg + ffprobe: macOS `brew install ffmpeg`, Ubuntu `sudo apt-get install -y ffmpeg`",
    "cargo-nextest": "cargo-nextest: `cargo install cargo-nextest --locked` or see https://get.nexte.st",
    "cargo-deny": "cargo-deny: `cargo install cargo-deny --locked`",
    "cargo-llvm-cov": "cargo-llvm-cov: `cargo install cargo-llvm-cov`",
}


def missing_for(steps):
    needed = []
    for name, _ in steps:
        for tool in STEP_TOOLS.get(name, []):
            if tool not in needed and shutil.which(tool) is None:
                needed.append(tool)
    return needed


def report_dependencies(root):
    print(f"\n=== deps: {' '.join(DEPS_CMD)} (report-only, never fails the run) ===", flush=True)
    try:
        result = subprocess.run(DEPS_CMD, cwd=root, capture_output=True, text=True)
    except OSError as err:
        print(f"dependency updates report skipped: {err}")
        return
    if result.returncode != 0:
        print("dependency updates report unavailable (offline or registry error); continuing")
        return
    updates = [
        line.strip()[len("Updating"):].strip()
        for line in (result.stdout + result.stderr).splitlines()
        if line.strip().startswith("Updating") and " -> " in line
    ]
    if updates:
        print("outdated dependencies found:")
        for update in updates:
            print(f"  {update}")
    else:
        print("all dependencies are up to date")


def main(argv):
    root = Path(__file__).resolve().parent
    selected = argv[1:]
    gate_names = [name for name, _ in STEPS]
    known = gate_names + ["deps"]
    unknown = [s for s in selected if s not in known]
    if unknown:
        print(f"unknown steps: {', '.join(unknown)}")
        print(f"available: {', '.join(known)}")
        return 2
    run_deps = not selected or "deps" in selected
    steps = [step for step in STEPS if not selected or step[0] in selected]
    missing = missing_for(steps)
    if missing:
        print(f"missing required tool(s): {', '.join(missing)}")
        for hint in dict.fromkeys(INSTALL_HINTS[tool] for tool in missing):
            print(f"install: {hint}")
        return 1
    for name, cmd in steps:
        print(f"\n=== {name}: {' '.join(cmd)} ===", flush=True)
        result = subprocess.run(cmd, cwd=root)
        if result.returncode != 0:
            print(f"\n{name} failed (exit {result.returncode})")
            return result.returncode
    if steps:
        print("\nall qa steps passed")
    if run_deps:
        report_dependencies(root)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
