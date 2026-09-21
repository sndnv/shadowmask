#!/usr/bin/env python3

import argparse
import fnmatch
import json
import logging
import os
import shutil
import subprocess
import sys
from pathlib import Path

DESCRIPTION = 'Refresh the vendored third-party license texts bundled by the enrichment build'

TARGET = 'licenses/enrichment'
CRATES = ['ct2rs', 'sentencepiece-sys']
PATTERNS = ['LICENSE*', 'COPYING*']
METADATA_CMD = ['cargo', 'metadata', '--locked', '--all-features', '--format-version', '1']
REFRESH_CMD = 'python3 licenses/refresh_licenses.py'


class Paths:
    def __init__(self):
        self.current = Path(os.path.realpath(__file__))
        self.repo = self.current.parents[1]
        self.target = self.repo / TARGET


def abort():
    sys.exit(1)


def crate_sources(repo):
    result = subprocess.run(METADATA_CMD, cwd=repo, capture_output=True, text=True)

    if result.returncode != 0:
        logging.error(
            'Resolving crate metadata failed with code [{}]: {}'.format(result.returncode, result.stderr.strip())
        )
        abort()

    packages = json.loads(result.stdout)['packages']
    sources = {}

    for crate in CRATES:
        matches = [Path(package['manifest_path']).parent for package in packages if package['name'] == crate]

        if len(matches) != 1:
            logging.error('Expected exactly one source for crate [{}] but found [{}]'.format(crate, len(matches)))
            abort()

        logging.debug('Resolved crate [{}] to [{}]'.format(crate, matches[0]))
        sources[crate] = matches[0]

    return sources


def is_license(name):
    return any(fnmatch.fnmatchcase(name.upper(), pattern) for pattern in PATTERNS)


def collect(sources):
    collected = {}

    for crate, source in sources.items():
        found = sorted(path for path in source.rglob('*') if path.is_file() and is_license(path.name))

        if not found:
            logging.error('No license texts found for crate [{}] in [{}]'.format(crate, source))
            abort()

        for path in found:
            collected[Path(crate) / path.relative_to(source)] = path

        logging.info('Collected [{}] license texts for crate [{}]'.format(len(found), crate))

    return collected


def refresh_licenses(paths, collected):
    if paths.target.is_dir():
        shutil.rmtree(paths.target)

    for relative, source in sorted(collected.items()):
        target = paths.target / relative

        logging.debug('Refreshing license text: {} -> {}'.format(source, target))

        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)

    logging.info('Refreshed [{}] license texts in [{}]'.format(len(collected), TARGET))


def verify_licenses(paths, collected):
    existing = set()

    if paths.target.is_dir():
        existing = {path.relative_to(paths.target) for path in paths.target.rglob('*') if path.is_file()}

    failures = ['missing [{}]'.format(relative) for relative in sorted(set(collected) - existing)]
    failures += ['unexpected [{}]'.format(relative) for relative in sorted(existing - set(collected))]
    failures += [
        'outdated [{}]'.format(relative)
        for relative in sorted(set(collected) & existing)
        if (paths.target / relative).read_bytes() != collected[relative].read_bytes()
    ]

    if failures:
        logging.error('Verification of [{}] failed with [{}] problem(s)'.format(TARGET, len(failures)))

        for failure in failures:
            logging.error(failure)

        logging.error('Refresh with: {}'.format(REFRESH_CMD))
        abort()

    logging.info('Verified [{}] license texts in [{}]'.format(len(collected), TARGET))


def main():
    paths = Paths()

    parser = argparse.ArgumentParser(description=DESCRIPTION)

    parser.add_argument(
        '-v', '--verbose',
        action='store_true',
        help='Enable debug logging'
    )

    parser.add_argument(
        '--verify',
        action='store_true',
        help='Verify that the distributed license texts match the vendored crate sources'
    )

    args = parser.parse_args()

    logging.basicConfig(
        format='[%(asctime)-15s] [%(levelname)s] [%(name)-5s]: %(message)s',
        level=logging.getLevelName(logging.DEBUG if args.verbose else logging.INFO)
    )

    os.chdir(paths.repo)

    collected = collect(crate_sources(paths.repo))

    if args.verify:
        verify_licenses(paths=paths, collected=collected)
    else:
        refresh_licenses(paths=paths, collected=collected)


if __name__ == '__main__':
    main()
