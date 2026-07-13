#!/usr/bin/env python3
import shutil
import subprocess
import sys
from pathlib import Path

FAIL_UNDER_LINES = "99.5"
IGNORE_COVERAGE = r"crates/server/src/main\.rs$"

STEPS = [
    ("fmt", ["cargo", "fmt", "--all", "--check"]),
    ("clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"]),
    ("build", ["cargo", "build", "--workspace", "--locked"]),
    ("deny", ["cargo", "deny", "check"]),
    ("coverage", ["cargo", "llvm-cov", "nextest", "--workspace", "--locked", "--ignore-filename-regex", IGNORE_COVERAGE, "--fail-under-lines", FAIL_UNDER_LINES]),
]

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


def main(argv):
    root = Path(__file__).resolve().parent
    selected = argv[1:]
    names = [name for name, _ in STEPS]
    unknown = [s for s in selected if s not in names]
    if unknown:
        print(f"unknown steps: {', '.join(unknown)}")
        print(f"available: {', '.join(names)}")
        return 2
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
    print("\nall qa steps passed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
