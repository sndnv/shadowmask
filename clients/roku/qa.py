#!/usr/bin/env python3
import re
import subprocess
import sys
from pathlib import Path

from package import IMAGE, RUNTIME_HINT, RUNTIMES, build_image, runtime

MANIFEST_KEYS = [
    "title",
    "major_version",
    "minor_version",
    "build_version",
    "ui_resolutions",
    "requires_mkv",
    "mm_icon_focus_hd",
    "mm_icon_focus_fhd",
    "splash_screen_hd",
    "splash_screen_fhd",
]

TEST_TIMEOUT_SECONDS = "120"

PASS_MARKER = "[Rooibos Result]: PASS"
FAIL_MARKER = "[Rooibos Result]: FAIL"


def in_container(engine, root, script, writable=False):
    mount = "rw" if writable else "ro"
    return [
        engine, "run", "--rm",
        "-v", f"{root}:/work:{mount}",
        "-w", "/work",
        IMAGE,
        "sh", "-c", script,
    ]


def lint(engine, root):
    return subprocess.run(in_container(
        engine, root,
        "bslint --project bsconfig.json && bslint --project bsconfig-test.json",
    )).returncode


def test(engine, root):
    result = subprocess.run(
        in_container(
            engine, root,
            "bsc --project bsconfig-test.json"
            f" && timeout {TEST_TIMEOUT_SECONDS} brs-cli --root build/test",
            writable=True,
        ),
        capture_output=True,
        text=True,
    )
    output = result.stdout + result.stderr
    print(output, end="" if output.endswith("\n") else "\n")
    if FAIL_MARKER in output:
        print("tests reported failures")
        return 1
    if PASS_MARKER not in output:
        print(f"no test result reported; expected '{PASS_MARKER}' in the output")
        print("the suite crashed, hung, or never started")
        return 1
    return 0


def xml(engine, root):
    return subprocess.run(in_container(
        engine, root,
        "xmllint --noout $(find components -name '*.xml')",
    )).returncode


COMPONENT_NAME = re.compile(r'<component\s[^>]*\bname="([^"]+)"')
COMPONENT_EXTENDS = re.compile(r'<component\s[^>]*\bextends="([^"]+)"')

SCOPE_EXEMPT = {"components/api/HttpTask.xml"}


def scope(engine, root):
    modules = sorted(
        path.name for path in (root / "source").glob("*.brs")
        if path.name != "main.brs"
    )
    components = sorted((root / "components").rglob("*.xml"))
    sources = {path: path.read_text() for path in components}
    defined = {
        match.group(1)
        for text in sources.values()
        if (match := COMPONENT_NAME.search(text))
    }

    failed = False
    inherited = 0
    for component, text in sources.items():
        extends = COMPONENT_EXTENDS.search(text)
        if extends and extends.group(1) in defined:
            inherited += 1
            continue
        imported = [
            module for module in modules
            if f'uri="pkg:/source/{module}"' in text
        ]
        if component.relative_to(root).as_posix() in SCOPE_EXEMPT:
            if imported:
                failed = True
                print(f"{component.relative_to(root)} is scope-exempt but imports: {', '.join(imported)}")
            continue
        missing = [module for module in modules if module not in imported]
        if missing:
            failed = True
            print(f"{component.relative_to(root)} does not import: {', '.join(missing)}")
    if failed:
        print("source/ is not in scope for a component that does not <script>-import it;")
        print("every component carries the whole set so a new call site cannot miss one,")
        print(f"except {', '.join(sorted(SCOPE_EXEMPT))}, which run off the render thread")
        print("and carry nothing, so a request thread does not parse the whole channel")
        return 1
    print(
        f"every component imports all {len(modules)} source modules "
        f"({inherited} inherit their scope from a parent component, "
        f"{len(SCOPE_EXEMPT)} carry none by design)"
    )
    return 0


def manifest(engine, root):
    text = (root / "manifest").read_text()
    present = [
        line.split("=", 1)[0].strip()
        for line in text.splitlines()
        if "=" in line and not line.lstrip().startswith("#")
    ]
    missing = [key for key in MANIFEST_KEYS if key not in present]
    if missing:
        print(f"manifest is missing required key(s): {', '.join(missing)}")
        return 1
    print(f"manifest carries {', '.join(MANIFEST_KEYS)}")
    return 0


STEPS = [
    ("image", "build the tooling image", build_image),
    ("lint", "bslint over both projects", lint),
    ("test", "rooibos under brs-cli", test),
    ("xml", "xmllint over components", xml),
    ("scope", "every component imports all of source/", scope),
    ("manifest", "required manifest keys", manifest),
]


def main(argv):
    root = Path(__file__).resolve().parent
    selected = argv[1:]
    known = [name for name, _, _ in STEPS]
    unknown = [step for step in selected if step not in known]
    if unknown:
        print(f"unknown steps: {', '.join(unknown)}")
        print(f"available: {', '.join(known)}")
        return 2

    engine = runtime()
    if engine is None:
        print(f"missing required tool(s): {' or '.join(RUNTIMES)}")
        print(f"install: {RUNTIME_HINT}")
        return 1
    print(f"container runtime: {engine}")

    steps = [step for step in STEPS if not selected or step[0] in selected]
    if "image" not in [name for name, _, _ in steps]:
        steps = [STEPS[0]] + steps

    for name, description, run in steps:
        print(f"\n=== {name}: {description} ===", flush=True)
        code = run(engine, root)
        if code != 0:
            print(f"\n{name} failed (exit {code})")
            return code

    print("\nall qa steps passed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
