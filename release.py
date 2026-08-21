#!/usr/bin/env python3

import argparse
import logging
import re
import semver
import subprocess
import sys

DESCRIPTION = ('Bumps the project version to the next release version, syncs the lockfile, '
               'commits and tags the changes, and updates to the next snapshot version')


def require_no_changes():
    result = subprocess.run(['git', 'status', '--porcelain'], capture_output=True, text=True)
    output = list(filter(None, result.stdout.split('\n') + result.stderr.split('\n')))

    if len(output) == 0:
        logging.debug('No changes found in repo')
    else:
        logging.error('Release failed - uncommitted changes found in repo [\n    {}\n]'.format('\n    '.join(output)))
        sys.exit(1)


def require_version_updated(next_version, updated_version):
    if updated_version == next_version:
        logging.debug('Version change applied')
    else:
        logging.error(
            'Release failed - expected updated version to be [{}] but [{}] found'.format(next_version, updated_version)
        )
        sys.exit(1)


def get_version_from(version_file, with_version_regex):
    with open(version_file) as f:
        pattern = re.compile(with_version_regex)
        match = next(filter(lambda m: m is not None, map(pattern.match, f.readlines())), None)

    if match:
        logging.debug('Loaded version [{}] from file [{}]'.format(match.group(1), version_file))
        return match.group(1)
    else:
        logging.error('Release failed - could not find version in [{}]'.format(version_file))
        sys.exit(1)


def get_current_version(version_files):
    versions = {vf: get_version_from(version_file=vf, with_version_regex=rx) for vf, rx in version_files.items()}
    unique_versions = list(set(versions.values()))

    if len(unique_versions) != 1:
        logging.error(
            'Release failed - found [{}] different versions ({}) in [\n    {}\n]'.format(
                len(unique_versions),
                ', '.join(unique_versions),
                '\n    '.join(map(lambda e: '[{}] in {}'.format(e[1], e[0]), versions.items()))
            )
        )
        sys.exit(1)
    else:
        return unique_versions[0]


def get_next_version(current_version, next_version):
    current = semver.Version.parse(current_version)

    computed = {
        'patch': current.bump_patch() if not current_version.endswith('-SNAPSHOT') else semver.Version.parse(
            current_version.replace('-SNAPSHOT', '')),
        'minor': current.bump_minor(),
        'major': current.bump_major(),
    }.get(next_version.lower())

    return str(computed or semver.Version.parse(next_version))


def apply_next_version_to(version_file, current_version, next_version, with_version_regex):
    pattern = re.compile(with_version_regex)
    with open(version_file, 'r') as f:
        updated = list(
            map(
                lambda line: line.replace(current_version, next_version) if pattern.match(line) else line,
                f.readlines()
            )
        )

    with open(version_file, 'w') as f:
        f.write(''.join(updated))


def apply_next_version(version_files, current_version, next_version):
    for version_file, version_regex in version_files.items():
        apply_next_version_to(
            version_file=version_file,
            current_version=current_version,
            next_version=next_version,
            with_version_regex=version_regex
        )


def exec_command(command):
    if subprocess.run(command).returncode == 0:
        logging.debug('Executed command [{}]'.format(' '.join(command)))
    else:
        logging.error('Release failed - could not execute command [{}]'.format(' '.join(command)))
        sys.exit(1)


def apply_extra_actions(actions):
    for target_file, action in actions.items():
        action(target_file)


def sync_cargo_lock(target_file):
    exec_command(command=['cargo', 'update', '--workspace'])
    exec_command(command=['git', 'add', target_file])


def commit_version_files(version_files, next_version):
    for version_file in version_files:
        exec_command(command=['git', 'add', version_file])
    exec_command(command=['git', 'commit', '-m', 'Updating version to {}'.format(next_version)])


def create_tag(next_version):
    exec_command(command=['git', 'tag', 'v{}'.format(next_version)])


def main():
    version_regex = r'\d+\.\d+\.\d+.*'

    version_files = {
        'Cargo.toml': r'^version = "({})"'.format(version_regex),
        'clients/flutter/pubspec.yaml': r'^version: ({})'.format(version_regex),
    }

    extra_actions = {
        'Cargo.lock': sync_cargo_lock,
    }

    parser = argparse.ArgumentParser(description=DESCRIPTION)

    parser.add_argument(
        '-n', '--next',
        required=False,
        default='patch',
        help='select next release version; can be either one of [major|minor|patch] or an explicit version (ex: 1.5.0)'
    )

    parser.add_argument(
        '-v', '--verbose',
        action='store_true',
        help='enable debug logging'
    )

    args = parser.parse_args()

    logging.basicConfig(
        format='[%(asctime)-15s] [%(levelname)s] [%(name)-5s]: %(message)s',
        level=logging.getLevelName(logging.DEBUG if args.verbose else logging.INFO)
    )

    require_no_changes()

    current_version = get_current_version(version_files=version_files)
    next_version = get_next_version(current_version=current_version, next_version=args.next)

    apply_next_version(version_files=version_files, current_version=current_version, next_version=next_version)
    apply_extra_actions(extra_actions)

    require_version_updated(next_version=next_version, updated_version=get_current_version(version_files=version_files))

    commit_version_files(version_files=version_files, next_version=next_version)
    create_tag(next_version=next_version)

    next_snapshot_version = '{}-SNAPSHOT'.format(get_next_version(current_version=next_version, next_version='patch'))

    apply_next_version(version_files=version_files, current_version=next_version, next_version=next_snapshot_version)
    apply_extra_actions(extra_actions)

    require_version_updated(
        next_version=next_snapshot_version,
        updated_version=get_current_version(version_files=version_files)
    )

    commit_version_files(version_files=version_files, next_version=next_snapshot_version)


if __name__ == '__main__':
    main()
