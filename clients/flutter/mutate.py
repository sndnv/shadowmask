#!/usr/bin/env python3

import glob
import os
import shutil
import signal
import subprocess
import sys
import tempfile

flutter_ui_path = os.path.dirname(os.path.realpath(__file__))

left_out_top = {'build', 'coverage', 'android', 'ios', 'macos', 'linux', 'windows'}

copy_prefix = 'shadowmask-mutation-'


def left_out(directory, names):
    if directory == flutter_ui_path:
        return [name for name in names if name in left_out_top]
    if directory == os.path.join(flutter_ui_path, '.dart_tool'):
        return [name for name in names if name == 'flutter_build']
    return []


def target_of(argv):
    if len(argv) != 2:
        return None
    path = os.path.relpath(os.path.realpath(argv[1]), flutter_ui_path)
    generated = path.endswith('.freezed.dart') or path.endswith('.g.dart')
    if not path.startswith('lib' + os.sep) or not path.endswith('.dart') or generated:
        return None
    return path if os.path.isfile(os.path.join(flutter_ui_path, path)) else None


def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def sweep():
    for path in glob.glob(os.path.join(tempfile.gettempdir(), copy_prefix + '*-*')):
        owner = os.path.basename(path)[len(copy_prefix):].split('-')[0]
        if owner.isdigit() and not alive(int(owner)):
            shutil.rmtree(path, ignore_errors=True)
            print('>: removed [{}], left by a run that is gone'.format(path), flush=True)


def hold_signals():
    for held in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(held, signal.SIG_IGN)


def stop(signum, frame):
    hold_signals()
    raise KeyboardInterrupt


target = target_of(sys.argv)
if target is None:
    print('>: usage: mutate.py lib/<path>.dart (one source file, not a generated one)')
    sys.exit(2)

for stopping in (signal.SIGTERM, signal.SIGHUP):
    signal.signal(stopping, stop)

sweep()
reports = os.path.join(flutter_ui_path, 'build', 'mutation', target[:-len('.dart')].replace(os.sep, '_'))
copy_root = tempfile.mkdtemp(prefix='{}{}-'.format(copy_prefix, os.getpid()))
copy = os.path.join(copy_root, 'flutter')
child = None
result = 1
try:
    print('>: copying the client to [{}]'.format(copy), flush=True)
    shutil.copytree(flutter_ui_path, copy, symlinks=True, ignore=left_out)
    child = subprocess.Popen(
        [
            'dart', 'run', 'mutation_test', '-b', '--rules', 'mutation_test.xml',
            target, '-f', 'all', '-o', reports
        ],
        cwd=copy,
        start_new_session=True
    )
    result = child.wait()
    print('>: mutation testing of [{}] finished with exit code [{}]'.format(target, result))
    print('>: report in [file://{}/mutation-test-report.html]'.format(reports))
except KeyboardInterrupt:
    hold_signals()
    result = 130
    if child is not None:
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        child.wait()
    print('>: mutation testing of [{}] interrupted'.format(target))
finally:
    shutil.rmtree(copy_root, ignore_errors=True)
    print('>: removed [{}]'.format(copy_root))

sys.exit(result)
