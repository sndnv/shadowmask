#!/usr/bin/env python3
import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

from package import DEFAULT_ARCHIVE, build_and_package

DEFAULT_USER = "rokudev"
DEFAULT_PORT = 80
PASSWORD_ENV = "ROKU_DEV_PASSWORD"
SIMULATOR = "simulator"
SIMULATOR_PASSWORD = "rokudev"

SUCCESS_MARKERS = ["Install Success", "Identical to previous version"]


def install(archive, host, port, user, password):
    url = f"http://{host}:{port}/plugin_install"
    print(f"=== installing to {url} ===", flush=True)
    result = subprocess.run(
        [
            "curl", "-sS", "--digest", "--user", f"{user}:{password}",
            "-F", "mysubmit=Install",
            "-F", f"archive=@{archive}",
            "-F", "passwd=",
            url,
        ],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(result.stderr.strip())
        return result.returncode

    body = result.stdout
    if any(marker in body for marker in SUCCESS_MARKERS):
        print(next(m for m in SUCCESS_MARKERS if m in body))
        return 0

    print("install failed; the device replied:")
    print(readable(body))
    return 1


def readable(body):
    text = []
    depth = 0
    for character in body:
        if character == "<":
            depth += 1
        elif character == ">":
            depth -= 1
        elif depth == 0:
            text.append(character)
    lines = [line.strip() for line in "".join(text).splitlines()]
    return "\n".join(line for line in lines if line) or body.strip()


def main(argv):
    root = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser(
        description="Package the Roku channel and install it on a device or the simulator.",
    )
    parser.add_argument(
        "--target", default=SIMULATOR,
        help="'simulator' for a local brs-desktop, or a device host/IP",
    )
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument("--user", default=DEFAULT_USER)
    parser.add_argument(
        "--password", default=os.environ.get(PASSWORD_ENV, ""),
        help=f"developer-mode password (or set {PASSWORD_ENV})",
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

    simulated = args.target == SIMULATOR
    host = "127.0.0.1" if simulated else args.target
    password = args.password or (SIMULATOR_PASSWORD if simulated else "")

    if shutil.which("curl") is None:
        print("missing required tool(s): curl")
        return 1

    archive = build_and_package(
        root,
        root / DEFAULT_ARCHIVE,
        skip_image=args.skip_image,
        skip_build=args.skip_build,
    )
    if archive is None:
        return 1

    return install(archive, host, args.port, args.user, password)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
