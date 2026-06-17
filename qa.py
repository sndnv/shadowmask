#!/usr/bin/env python3
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
