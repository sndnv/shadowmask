#!/usr/bin/env python3

import os
import resource
import subprocess
import sys

flutter_ui_path = os.path.dirname(os.path.realpath(__file__))

wanted_open_files = 4096


def raise_open_file_limit():
    soft, hard = resource.getrlimit(resource.RLIMIT_NOFILE)
    if soft >= wanted_open_files:
        return
    target = wanted_open_files if hard == resource.RLIM_INFINITY \
        else min(wanted_open_files, hard)
    try:
        resource.setrlimit(resource.RLIMIT_NOFILE, (target, hard))
    except (OSError, ValueError):
        print('>: could not raise the open file limit from [{}]'.format(soft))
        return
    print('>: raised the open file limit from [{}] to [{}]'.format(soft, target))


def run_command(command, description):
    result = subprocess.run(command).returncode
    if result != 0:
        print('>: {} failed with exit code [{}]'.format(description, result))
        sys.exit(result)


raise_open_file_limit()

run_command(
    command=['flutter', 'pub', 'get'],
    description='Getting packages'
)

run_command(
    command=['dart', 'run', 'build_runner', 'build'],
    description='Build'
)

run_command(
    command=['dart', 'format', '--output=none', '--set-exit-if-changed', 'lib', 'test'],
    description='Format check'
)

run_command(
    command=['flutter', 'analyze'],
    description='Code linting'
)

test_result = subprocess.run(['flutter', 'test', '--coverage']).returncode
print('>: Testing finished with exit code [{}]'.format(test_result))

if test_result == 0:
    target = '{}/coverage/html'.format(flutter_ui_path)
    try:
        coverage_result = subprocess.run(
            [
                'genhtml', '{}/coverage/lcov.info'.format(flutter_ui_path),
                '-o', target
            ]
        ).returncode
        if coverage_result == 0:
            print('>: Coverage written to [file://{}/{}]'.format(target, 'index.html'))
        print('>: Code coverage finished with exit code [{}]'.format(coverage_result))
    except FileNotFoundError:
        print('>: genhtml not found (install lcov for HTML coverage); skipping report')

sys.exit(test_result)
