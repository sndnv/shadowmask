#!/usr/bin/env python3

import os
import subprocess
import sys

dev_path = os.path.dirname(os.path.realpath(__file__))
browser_dir = '{}/build/browser/cache'.format(dev_path)
ui_url = os.environ.get('SHADOWMASK_UI_URL', 'http://localhost:8090')


def run_command(command, description):
    try:
        result = subprocess.run(command).returncode
        if result != 0:
            print('>: {} failed with exit code [{}]'.format(description, result))
            sys.exit(result)
    except KeyboardInterrupt:
        sys.exit(0)


print('>: Opening a dev browser with a throwaway profile at [{}]'.format(ui_url))

if sys.platform == 'linux':
    run_command(
        command=[
            'google-chrome',
            '--user-data-dir={}'.format(browser_dir),
            ui_url
        ],
        description='Opening browser'
    )
elif sys.platform == 'darwin':
    run_command(
        command=[
            'open', '-na', 'Google Chrome',
            '--args',
            '--user-data-dir={}'.format(browser_dir),
            ui_url
        ],
        description='Opening browser'
    )
else:
    print('>: Platform [{}] is not supported'.format(sys.platform))
    sys.exit(1)
