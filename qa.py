#!/usr/bin/env python3
import shutil
import subprocess
import sys
from pathlib import Path

STEPS = [
    ("fmt", ["cargo", "fmt", "--all", "--check"]),
    ("clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
    ("build", ["cargo", "build", "--workspace"]),
    ("test", ["cargo", "test", "--workspace"]),
    ("coverage", ["cargo", "llvm-cov", "--workspace"]),
]

REQUIRED_TOOLS = ["ffmpeg", "ffprobe"]
TOOL_DEPENDENT_STEPS = {"test", "coverage"}


def missing_tools():
    return [tool for tool in REQUIRED_TOOLS if shutil.which(tool) is None]


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
    if any(name in TOOL_DEPENDENT_STEPS for name, _ in steps):
        missing = missing_tools()
        if missing:
            print(f"missing required tool(s): {', '.join(missing)}")
            print("ffmpeg and ffprobe are required for the media probe and smoke tests.")
            print("install — macOS: `brew install ffmpeg` · Ubuntu: `sudo apt-get install -y ffmpeg`")
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
