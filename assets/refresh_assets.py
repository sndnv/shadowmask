#!/usr/bin/env python3

import argparse
import logging
import os
import subprocess
import sys
from pathlib import Path

DESCRIPTION = 'Refresh shared assets used by client subprojects'


class Paths:
    def __init__(self):
        self.current = Path(os.path.realpath(__file__))
        self.repo = self.current.parents[1]
        self.assets = '{}/assets'.format(self.repo)


def abort():
    sys.exit(1)


def copy_asset(paths, project_name, asset):
    asset_path = '{}/{}'.format(paths.assets, asset['asset'])
    target_path = '{}/{}/{}'.format(paths.repo, project_name, asset['target'])

    logging.debug('Refreshing asset for [{}]: {} -> {}'.format(project_name, asset_path, target_path))

    os.makedirs(os.path.dirname(target_path), exist_ok=True)

    copy_result = subprocess.run(['cp', asset_path, target_path]).returncode

    if copy_result == 0:
        logging.info('Refreshed asset [{}] for [{}]'.format(asset['asset'], project_name))
    else:
        logging.error(
            'Refresh of asset [{}] targeting [{}] for [{}] failed with code [{}]'.format(
                asset['asset'], target_path, project_name, copy_result
            )
        )
        abort()


def verify_asset(paths, project_name, asset):
    asset_path = '{}/{}'.format(paths.assets, asset['asset'])
    target_path = '{}/{}/{}'.format(paths.repo, project_name, asset['target'])

    logging.debug('Verifying asset for [{}]: {} -> {}'.format(project_name, asset_path, target_path))

    compare_result = subprocess.run(['cmp', '-s', asset_path, target_path]).returncode

    if compare_result == 0:
        logging.info('Verified asset [{}] for [{}]'.format(asset['asset'], project_name))
    else:
        logging.error(
            'Verification of asset [{}] targeting [{}] for [{}] failed with code [{}]'.format(
                asset['asset'], target_path, project_name, compare_result
            )
        )
        abort()


def main():
    paths = Paths()

    targets = {
        'clients/basic': [
            {'asset': 'shadowmask.logo.svg', 'target': 'favicon.svg'},
            {'asset': 'shadowmask.placeholder.svg', 'target': 'placeholder.svg'},
        ],
    }

    parser = argparse.ArgumentParser(description=DESCRIPTION)

    parser.add_argument(
        '-p', '--project',
        required=False,
        choices=targets.keys(),
        help='Specific project for which to refresh assets'
    )

    parser.add_argument(
        '-v', '--verbose',
        action='store_true',
        help='Enable debug logging'
    )

    parser.add_argument(
        '--verify',
        action='store_true',
        help='Verify that all distributed assets match the central source'
    )

    args = parser.parse_args()

    logging.basicConfig(
        format='[%(asctime)-15s] [%(levelname)s] [%(name)-5s]: %(message)s',
        level=logging.getLevelName(logging.DEBUG if args.verbose else logging.INFO)
    )

    os.chdir(paths.repo)

    projects = [args.project] if args.project else list(targets.keys())

    for current_project in projects:
        for current_asset in targets[current_project]:
            if args.verify:
                verify_asset(paths=paths, project_name=current_project, asset=current_asset)
            else:
                copy_asset(paths=paths, project_name=current_project, asset=current_asset)


if __name__ == '__main__':
    main()
