#!/usr/bin/env python3
import argparse
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

IMAGE = "shadowmask-roku-tooling:latest"
RUNTIMES = ["podman", "docker"]

RUNTIME_HINT = (
    "a container runtime: macOS `brew install podman && podman machine init && "
    "podman machine start`, Ubuntu `sudo apt-get install -y podman`"
)

STAGING = Path("build") / "app"
DEFAULT_ARCHIVE = Path("build") / "shadowmask-roku.zip"


def runtime():
    for candidate in RUNTIMES:
        if shutil.which(candidate) is not None:
            return candidate
    return None


def build_image(engine, root):
    return subprocess.run(
        [engine, "build", "-t", IMAGE, "-f", "Containerfile", "."],
        cwd=root / "tooling",
    ).returncode


def build(engine, root):
    return subprocess.run([
        engine, "run", "--rm",
        "-v", f"{root}:/work:rw",
        "-w", "/work",
        IMAGE,
        "bsc", "--project", "bsconfig.json",
    ]).returncode


def package(staging, archive):
    if not staging.is_dir():
        print(f"nothing staged at [{staging}]; build first")
        return None
    if not (staging / "manifest").is_file():
        print(f"no manifest at the root of [{staging}]")
        return None

    archive.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as bundle:
        for path in sorted(staging.rglob("*")):
            if path.is_file():
                bundle.write(path, path.relative_to(staging))
    print(f"packaged [{archive}] ({archive.stat().st_size} bytes)")
    return archive


def build_and_package(root, archive, skip_image=False, skip_build=False):
    if not skip_build:
        engine = runtime()
        if engine is None:
            print(f"missing required tool(s): {' or '.join(RUNTIMES)}")
            print(f"install: {RUNTIME_HINT}")
            return None

        if not skip_image:
            print("=== building the tooling image ===", flush=True)
            code = build_image(engine, root)
            if code != 0:
                print(f"image build failed (exit {code})")
                return None

        print("=== building the channel ===", flush=True)
        code = build(engine, root)
        if code != 0:
            print(f"build failed (exit {code})")
            return None

    return package(root / STAGING, archive)


def main(argv):
    root = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser(
        description="Build the Roku channel and package it into a sideloadable zip.",
    )
    parser.add_argument(
        "--output", default=None,
        help=f"archive path (default {DEFAULT_ARCHIVE})",
    )
    parser.add_argument(
        "--skip-image", action="store_true",
        help="use the tooling image as it already exists",
    )
    parser.add_argument(
        "--skip-build", action="store_true",
        help="package whatever is already staged",
    )
    args = parser.parse_args(argv[1:])

    archive = Path(args.output).resolve() if args.output else root / DEFAULT_ARCHIVE
    built = build_and_package(
        root,
        archive,
        skip_image=args.skip_image,
        skip_build=args.skip_build,
    )
    return 0 if built else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
