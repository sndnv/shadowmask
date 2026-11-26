#!/usr/bin/env python3

import argparse
import hashlib
import io
import json
import logging
import os
import re
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
from pathlib import Path

DESCRIPTION = 'Refresh the third-party notices for the libmpv bundled by the Windows desktop client'

RELEASE = '20261123'
BUILDER = 'media-kit/libmpv-win32-video-cmake'
BUILD_URL = 'https://github.com/{}/releases/download/{}/build-x86_64.json'.format(BUILDER, RELEASE)
TARGET = 'licenses/libmpv-windows.NOTICES.txt'
ABOUT_CONFIG = 'licenses/libmpv-windows.about.toml'
STD_ABOUT_CONFIG = 'licenses/libmpv-windows-std.about.toml'
ABOUT_TEMPLATE = 'licenses/libmpv-windows.hbs'
STD_FEATURES = 'backtrace,panic-unwind'
USER_AGENT = 'shadowmask-refresh-windows-notices'
REFRESH_CMD = 'python3 licenses/refresh_windows_notices.py'
RULE = '=' * 80


def github(repo):
    return {
        'raw': 'https://raw.githubusercontent.com/' + repo + '/{commit}/{path}',
        'home': 'https://github.com/' + repo,
        'git': 'https://github.com/' + repo + '.git',
    }


def gitlab(project):
    return {
        'raw': 'https://' + project + '/-/raw/{commit}/{path}',
        'home': 'https://' + project,
        'git': 'https://' + project + '.git',
    }


def tarball(url, sha256, prefix):
    return {'tarball': url, 'sha256': sha256, 'prefix': prefix, 'home': url}


COMPONENTS = [
    {
        'name': 'mpv', 'license': 'LGPL-2.1-or-later',
        'source': github('mpv-player/mpv'), 'dependency': 'mpv',
        'files': ['Copyright', 'LICENSE.LGPL'],
    },
    {
        'name': 'FFmpeg', 'license': 'LGPL-3.0-or-later',
        'source': github('FFmpeg/FFmpeg'), 'dependency': 'ffmpeg',
        'files': ['LICENSE.md', 'COPYING.LGPLv3', 'COPYING.GPLv3'],
    },
    {
        'name': 'dav1d', 'license': 'BSD-2-Clause',
        'source': gitlab('code.videolan.org/videolan/dav1d'), 'dependency': 'dav1d',
        'files': ['COPYING'],
    },
    {
        'name': 'libvpx', 'license': 'BSD-3-Clause',
        'source': github('webmproject/libvpx'), 'dependency': 'libvpx',
        'files': ['LICENSE', 'PATENTS'],
    },
    {
        'name': 'uavs3d', 'license': 'BSD-3-Clause',
        'source': github('uavs3/uavs3d'), 'dependency': 'uavs3d',
        'files': ['COPYING'],
    },
    {
        'name': 'libjxl', 'license': 'BSD-3-Clause',
        'source': github('libjxl/libjxl'), 'dependency': 'libjxl',
        'files': ['LICENSE', 'PATENTS'],
    },
    {
        'name': 'Highway', 'license': 'Apache-2.0 OR BSD-3-Clause, taken here under Apache-2.0',
        'source': github('google/highway'), 'dependency': 'highway',
        'files': ['LICENSE'],
    },
    {
        'name': 'Brotli', 'license': 'MIT',
        'source': github('google/brotli'), 'dependency': 'brotli',
        'files': ['LICENSE'],
    },
    {
        'name': 'libopenmpt', 'license': 'BSD-3-Clause',
        'source': tarball(
            'https://lib.openmpt.org/files/libopenmpt/src/libopenmpt-0.8.10+release.autotools.tar.gz',
            '976d259288c40cd285771f77d09eff3d02514511d5677ecfcfb53be5013317e2',
            'libopenmpt-0.8.10+release.autotools',
        ),
        'version': '0.8.10',
        'files': ['LICENSE'],
    },
    {
        'name': 'libogg', 'license': 'BSD-3-Clause',
        'source': github('xiph/ogg'), 'dependency': 'ogg',
        'files': ['COPYING'],
    },
    {
        'name': 'libvorbis', 'license': 'BSD-3-Clause',
        'source': github('xiph/vorbis'), 'dependency': 'vorbis',
        'files': ['COPYING'],
    },
    {
        'name': 'libmodplug', 'license': 'Public domain',
        'source': github('Konstanty/libmodplug'), 'dependency': 'libmodplug',
        'files': ['COPYING'],
    },
    {
        'name': 'libsoxr', 'license': 'LGPL-2.1-or-later',
        'source': gitlab('gitlab.com/shinchiro/soxr'), 'dependency': 'libsoxr',
        'files': ['LICENCE', 'COPYING.LGPL', 'src/pffft.c'], 'comments': ['src/pffft.c'],
    },
    {
        'name': 'libplacebo', 'license': 'LGPL-2.1-or-later',
        'source': github('haasn/libplacebo'), 'dependency': 'libplacebo',
        'files': ['LICENSE'],
    },
    {
        'name': 'shaderc', 'license': 'Apache-2.0',
        'source': github('google/shaderc'), 'dependency': 'shaderc',
        'files': ['LICENSE'],
    },
    {
        'name': 'glslang', 'license': 'Several, listed below',
        'source': github('KhronosGroup/glslang'), 'dependency': 'glslang',
        'files': ['LICENSE.txt'],
    },
    {
        'name': 'SPIRV-Tools', 'license': 'Apache-2.0',
        'source': github('KhronosGroup/SPIRV-Tools'), 'dependency': 'spirv-tools',
        'files': ['LICENSE'],
    },
    {
        'name': 'SPIRV-Headers', 'license': 'Several, listed below',
        'source': github('KhronosGroup/SPIRV-Headers'), 'dependency': 'spirv-headers',
        'files': ['LICENSE'],
    },
    {
        'name': 'SPIRV-Cross', 'license': 'Apache-2.0',
        'source': github('KhronosGroup/SPIRV-Cross'), 'dependency': 'spirv-cross',
        'patched': '57e4efcc84112abcd426dc5ec248b5339bee04f2',
        'files': ['LICENSE'],
    },
    {
        'name': 'Vulkan-Headers', 'license': 'Apache-2.0 OR MIT',
        'source': github('KhronosGroup/Vulkan-Headers'), 'dependency': 'vulkan-header',
        'files': ['LICENSE.md'],
    },
    {
        'name': 'Little CMS', 'license': 'MIT',
        'source': github('mm2/Little-CMS'), 'dependency': 'lcms2',
        'files': ['LICENSE'],
    },
    {
        'name': 'zimg', 'license': 'WTFPL',
        'source': github('sekrit-twc/zimg'), 'dependency': 'libzimg',
        'files': ['COPYING'],
    },
    {
        'name': 'graphengine', 'license': 'WTFPL',
        'source': github('sekrit-twc/graphengine'), 'dependency': 'graphengine',
        'files': ['COPYING'],
    },
    {
        'name': 'libass', 'license': 'ISC',
        'source': github('libass/libass'), 'dependency': 'libass',
        'files': ['COPYING'],
    },
    {
        'name': 'GNU FriBidi', 'license': 'LGPL-2.1-or-later',
        'source': github('fribidi/fribidi'), 'dependency': 'fribidi',
        'files': ['COPYING'],
    },
    {
        'name': 'HarfBuzz', 'license': 'Old MIT',
        'source': github('harfbuzz/harfbuzz'), 'dependency': 'harfbuzz',
        'files': ['COPYING'],
    },
    {
        'name': 'FreeType', 'license': 'FTL OR GPL-2.0-or-later, taken here under FTL',
        'source': github('freetype/freetype'), 'dependency': 'freetype2',
        'files': ['LICENSE.TXT', 'docs/FTL.TXT', 'src/bdf/README', 'src/pcf/README'],
    },
    {
        'name': 'Fontconfig', 'license': 'Several, listed below',
        'source': gitlab('gitlab.freedesktop.org/fontconfig/fontconfig'), 'dependency': 'fontconfig',
        'patched': '3e9035e505c675dcbfcf2e4bdaae69b1313b1d5b',
        'files': ['COPYING'],
    },
    {
        'name': 'libunibreak', 'license': 'Zlib',
        'source': github('adah1972/libunibreak'), 'dependency': 'libunibreak',
        'files': ['LICENCE'],
    },
    {
        'name': 'uchardet',
        'license': 'MPL-1.1 OR GPL-2.0-or-later OR LGPL-2.1-or-later, taken here under LGPL-2.1-or-later',
        'source': gitlab('gitlab.freedesktop.org/uchardet/uchardet'), 'dependency': 'uchardet',
        'files': ['COPYING'],
    },
    {
        'name': 'GNU libiconv', 'license': 'LGPL-2.1-or-later',
        'source': tarball(
            'https://ftp.gnu.org/pub/gnu/libiconv/libiconv-1.19.1.tar.gz',
            '9a06844846d664d7122161700c10a64480d7c7bcead3091925a785b38c91edab',
            'libiconv-1.19.1',
        ),
        'version': '1.19.1',
        'files': ['COPYING.LIB'],
    },
    {
        'name': 'libva', 'license': 'MIT',
        'source': github('intel/libva'), 'dependency': 'libva',
        'files': ['COPYING'],
    },
    {
        'name': 'Intel VPL', 'license': 'MIT',
        'source': github('intel/libvpl'), 'dependency': 'libvpl',
        'files': ['LICENSE'],
    },
    {
        'name': 'AMF', 'license': 'MIT',
        'source': github('GPUOpen-LibrariesAndSDKs/AMF'), 'dependency': 'amf-headers',
        'files': ['LICENSE.txt'],
    },
    {
        'name': 'nv-codec-headers', 'license': 'MIT',
        'source': github('FFmpeg/nv-codec-headers'), 'dependency': 'nvcodec-headers',
        'files': ['include/ffnvcodec/dynlink_loader.h'], 'comments': ['include/ffnvcodec/dynlink_loader.h'],
    },
    {
        'name': 'xxHash', 'license': 'BSD-2-Clause',
        'source': github('Cyan4973/xxHash'), 'dependency': 'xxhash',
        'files': ['LICENSE'],
    },
    {
        'name': 'VapourSynth headers', 'license': 'LGPL-2.1-or-later',
        'source': github('vapoursynth/vapoursynth'), 'dependency': 'vapoursynth',
        'files': ['COPYING.LESSER'],
    },
    {
        'name': 'OpenSSL', 'license': 'Apache-2.0',
        'source': github('openssl/openssl'), 'dependency': 'openssl',
        'patched': '888fa4ed34b478cb4c39eef0281b9c5e7e223227',
        'files': ['LICENSE.txt'],
    },
    {
        'name': 'curl', 'license': 'curl',
        'source': github('curl/curl'), 'dependency': 'curl',
        'patched': '64d03b252ec62d0c8788cfd27bbd2c5cce639ca2',
        'files': ['COPYING'],
    },
    {
        'name': 'nghttp2', 'license': 'MIT',
        'source': github('nghttp2/nghttp2'), 'dependency': 'nghttp2',
        'files': ['COPYING'],
    },
    {
        'name': 'nghttp3', 'license': 'MIT',
        'source': github('ngtcp2/nghttp3'), 'dependency': 'nghttp3',
        'files': ['COPYING'],
    },
    {
        'name': 'ngtcp2', 'license': 'MIT',
        'source': github('ngtcp2/ngtcp2'), 'dependency': 'ngtcp2',
        'files': ['COPYING'],
    },
    {
        'name': 'c-ares', 'license': 'MIT',
        'source': github('c-ares/c-ares'), 'dependency': 'c-ares',
        'files': ['LICENSE.md'],
    },
    {
        'name': 'libpsl', 'license': 'MIT',
        'source': github('rockdaboot/libpsl'), 'dependency': 'libpsl',
        'files': ['LICENSE'],
    },
    {
        'name': 'Public Suffix List', 'license': 'MPL-2.0',
        'source': github('publicsuffix/list'),
        'submodule': {'repo': 'rockdaboot/libpsl', 'dependency': 'libpsl', 'path': 'list'},
        'files': ['LICENSE'],
    },
    {
        'name': 'libssh', 'license': 'LGPL-2.1-or-later',
        'source': gitlab('gitlab.com/libssh/libssh-mirror'), 'dependency': 'libssh',
        'files': ['COPYING'],
    },
    {
        'name': 'SRT', 'license': 'MPL-2.0',
        'source': github('Haivision/srt'), 'dependency': 'libsrt',
        'files': ['LICENSE'],
    },
    {
        'name': 'libxml2', 'license': 'MIT',
        'source': github('GNOME/libxml2'), 'dependency': 'libxml2',
        'files': ['Copyright'],
    },
    {
        'name': 'libarchive', 'license': 'Several, listed below',
        'source': github('libarchive/libarchive'), 'dependency': 'libarchive',
        'files': ['COPYING'],
    },
    {
        'name': 'libjpeg-turbo', 'license': 'IJG AND BSD-3-Clause AND Zlib',
        'source': github('libjpeg-turbo/libjpeg-turbo'), 'dependency': 'libjpeg',
        'files': ['LICENSE.md', 'README.ijg'],
    },
    {
        'name': 'libpng', 'license': 'libpng-2.0',
        'source': github('glennrp/libpng'), 'dependency': 'libpng',
        'files': ['LICENSE'],
    },
    {
        'name': 'zlib-ng', 'license': 'Zlib',
        'source': github('zlib-ng/zlib-ng'), 'dependency': 'zlib',
        'files': ['LICENSE.md'],
    },
    {
        'name': 'Zstandard', 'license': 'BSD-3-Clause OR GPL-2.0-only, taken here under BSD-3-Clause',
        'source': github('facebook/zstd'), 'dependency': 'zstd',
        'files': ['LICENSE'],
    },
    {
        'name': 'XZ Utils (liblzma)', 'license': '0BSD',
        'source': github('tukaani-project/xz'), 'dependency': 'xz',
        'files': ['COPYING', 'COPYING.0BSD'],
    },
    {
        'name': 'bzip2', 'license': 'bzip2-1.0.6',
        'source': gitlab('gitlab.com/bzip2/bzip2'), 'dependency': 'bzip2',
        'files': ['COPYING'],
    },
    {
        'name': 'MinGW-w64 runtime and winpthreads', 'license': 'Several, listed below',
        'source': github('mingw-w64/mingw-w64'),
        'unrecorded': {
            'release': '20261123',
            'commit': '219a392f34146fea03a88dbf197eb387f3928adb',
            'branch': 'master',
        },
        'files': [
            'COPYING.MinGW-w64-runtime/COPYING.MinGW-w64-runtime.txt',
            'mingw-w64-libraries/winpthreads/COPYING',
        ],
    },
    {
        'name': 'LLVM libc++, libc++abi and libunwind', 'license': 'Apache-2.0 WITH LLVM-exception',
        'source': github('llvm/llvm-project'), 'llvm': True,
        'files': ['libcxx/LICENSE.TXT', 'libcxxabi/LICENSE.TXT', 'libunwind/LICENSE.TXT'],
    },
    {
        'name': 'C++/WinRT', 'license': 'MIT',
        'source': github('microsoft/cppwinrt'),
        'unrecorded': {
            'release': '20261123',
            'commit': '9bb8a57e46aea546b70b0af73e2b4137b8fed759',
            'branch': 'master',
        },
        'files': ['LICENSE'],
    },
]


class Paths:
    def __init__(self):
        self.current = Path(os.path.realpath(__file__))
        self.repo = self.current.parents[1]
        self.target = self.repo / TARGET


def abort():
    sys.exit(1)


def fetch(url):
    request = urllib.request.Request(url, headers={'User-Agent': USER_AGENT})

    try:
        with urllib.request.urlopen(request, timeout=120) as response:
            return response.read()
    except OSError as error:
        logging.error('Fetching [{}] failed: [{}]'.format(url, error))
        abort()


def fetch_json(url):
    return json.loads(fetch(url))


def text_of(data, origin):
    try:
        text = data.decode('utf-8')
    except UnicodeDecodeError:
        text = data.decode('latin-1')

    if re.match(r'\s*<(!doctype html|html)', text, re.IGNORECASE):
        logging.error('[{}] returned an HTML page instead of a licence text'.format(origin))
        abort()

    text = text.replace('\r\n', '\n').strip('\n')

    if not re.search(r'\s', text):
        logging.error('[{}] is empty or a single word, like a symlink target: [{}]'.format(origin, text))
        abort()

    return text


def licence_comment(text, origin):
    for match in re.finditer(r'/\*.*?\*/', text, re.DOTALL):
        if 'Copyright' in match.group(0):
            return match.group(0)

    logging.error('[{}] has no comment with a copyright notice'.format(origin))
    abort()


def run(command, cwd=None, env=None):
    result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True)

    if result.returncode != 0:
        logging.error('[{}] failed with code [{}]: {}'.format(' '.join(command), result.returncode, result.stderr.strip()))
        abort()

    return result.stdout


def recorded(build, key):
    commit = build['dependencies'].get(key)

    if not commit:
        logging.error('Release [{}] records no commit for [{}]'.format(RELEASE, key))
        abort()

    return commit


def build_started(build):
    run_info = fetch_json('https://api.github.com/repos/{}/actions/runs/{}'.format(BUILDER, build['run_id']))
    return run_info['run_started_at']


def patched_commit(component, build, workspace):
    key = component['dependency']
    upstream = component['patched']
    tree = build['dependency_trees'].get(key)
    names = sorted(name for name in build['patches'] if re.match(r'{}-\d{{4}}-'.format(re.escape(key)), name))

    if not tree or not names:
        logging.error('Release [{}] records no source tree or no patches for [{}]'.format(RELEASE, key))
        abort()

    directory = workspace / 'patched' / key
    checkout(component['source']['git'], upstream, directory)
    identity = ['-c', 'user.name=refresh', '-c', 'user.email=refresh@localhost']

    for name in names:
        data = fetch('https://raw.githubusercontent.com/{}/{}/packages/{}'.format(BUILDER, build['source_sha'], name))

        if hashlib.sha256(data).hexdigest() != build['patches'][name]:
            logging.error('Patch [{}] does not match its recorded SHA-256 [{}]'.format(name, build['patches'][name]))
            abort()

        patch = workspace / name
        patch.write_bytes(data)
        run(['git', '-C', str(directory)] + identity + ['am', '-q', '--3way', str(patch)])

    applied = run(['git', '-C', str(directory), 'rev-parse', 'HEAD^{tree}']).strip()

    if applied != tree:
        logging.error('[{}] patched onto [{}] gives tree [{}], the release records [{}]; find its upstream commit again'.format(
            component['name'], upstream, applied, tree
        ))
        abort()

    note = (
        'The release patches this project ({}), so its recorded commit [{}] is not upstream. '
        'The text is from [{}]: those patches applied to it reproduce the recorded source tree [{}].'
    ).format(', '.join(names), recorded(build, key), upstream, tree)
    return upstream, note


def resolve(component, build, started, workspace):
    if 'patched' in component:
        return patched_commit(component, build, workspace)

    if 'unrecorded' in component:
        unrecorded = component['unrecorded']

        if unrecorded['release'] != RELEASE:
            logging.error('[{}] was resolved for release [{}]; find its commit again'.format(
                component['name'], unrecorded['release']
            ))
            abort()

        note = (
            'The release does not record this commit. '
            'The text is from [{}], the head of [{}] when the release build started ({}).'
        ).format(unrecorded['commit'], unrecorded['branch'], started)
        return unrecorded['commit'], note

    if 'submodule' in component:
        submodule = component['submodule']
        parent = recorded(build, submodule['dependency'])
        contents = fetch_json('https://api.github.com/repos/{}/contents/{}?ref={}'.format(
            submodule['repo'], submodule['path'], parent
        ))
        note = 'The [{}] submodule of {} at [{}], built into libpsl.'.format(
            submodule['path'], submodule['repo'], parent
        )
        return contents['sha'], note

    if component.get('llvm'):
        match = re.search(r'llvm-project\.git ([0-9a-f]{40})', build['toolchain']['clang'])

        if not match:
            logging.error('Release [{}] records no LLVM commit'.format(RELEASE))
            abort()

        return match.group(1), None

    if 'dependency' in component:
        return recorded(build, component['dependency']), None

    return None, None


def tarball_texts(component):
    source = component['source']
    data = fetch(source['tarball'])

    if hashlib.sha256(data).hexdigest() != source['sha256']:
        logging.error('[{}] does not match its SHA-256 [{}]'.format(source['tarball'], source['sha256']))
        abort()

    texts = []

    with tarfile.open(fileobj=io.BytesIO(data), mode='r:*') as archive:
        for path in component['files']:
            member = '{}/{}'.format(source['prefix'], path)

            try:
                extracted = archive.extractfile(member)
            except KeyError:
                extracted = None

            if extracted is None:
                logging.error('[{}] has no [{}]'.format(source['tarball'], member))
                abort()

            texts.append((path, text_of(extracted.read(), member)))

    return texts


def repository_texts(component, commit):
    texts = []

    for path in component['files']:
        url = component['source']['raw'].format(commit=commit, path=path)
        text = text_of(fetch(url), url)

        if path in component.get('comments', []):
            text = licence_comment(text, url)

        texts.append((path, text))

    return texts


def section(name, license_name, home, version, note, texts):
    lines = [RULE, name, 'License: ' + license_name, 'Source: ' + home, 'Version: ' + version]

    if note:
        lines.append('Note: ' + note)

    lines += [RULE, '']

    for path, text in texts:
        lines += ['--- {} ---'.format(path), '', text, '']

    return '\n'.join(lines)


def component_section(component, build, started, workspace):
    commit, note = resolve(component, build, started, workspace)

    if 'tarball' in component['source']:
        texts = tarball_texts(component)
        version = component['version']
    else:
        texts = repository_texts(component, commit)
        version = commit

    logging.info('Collected [{}] text(s) for [{}]'.format(len(texts), component['name']))

    return section(component['name'], component['license'], component['source']['home'], version, note, texts)


def checkout(url, commit, directory, sparse=None):
    env = dict(os.environ, GIT_LFS_SKIP_SMUDGE='1')
    commands = [['git', 'init', '-q', str(directory)], ['git', '-C', str(directory), 'remote', 'add', 'origin', url]]

    if sparse:
        commands.append(['git', '-C', str(directory), 'sparse-checkout', 'set', '--no-cone'] + sparse)

    commands += [
        ['git', '-C', str(directory), 'fetch', '-q', '--depth', '1', '--filter=blob:none', 'origin', commit],
        ['git', '-C', str(directory), 'checkout', '-q', 'FETCH_HEAD'],
    ]

    for command in commands:
        run(command, env=env)


def cargo_about(paths, directory, config, manifest, features=None, bootstrap=False):
    command = ['cargo', 'about', 'generate', '--locked', '--fail', '-c', str(paths.repo / config), '-m', manifest]

    if features:
        command += ['--features', features]

    command.append(str(paths.repo / ABOUT_TEMPLATE))
    env = dict(os.environ, RUSTC_BOOTSTRAP='1') if bootstrap else None

    return run(command, cwd=directory, env=env).strip('\n')


def subrandr_section(paths, build, workspace):
    commit = recorded(build, 'subrandr')
    directory = workspace / 'subrandr'
    checkout('https://github.com/afishhh/subrandr.git', commit, directory)
    crates = cargo_about(paths, directory, ABOUT_CONFIG, 'Cargo.toml')

    logging.info('Collected the Rust crates linked through [subrandr]')

    return section(
        'subrandr',
        'MPL-2.0, the Rust crates it links as listed below',
        'https://github.com/afishhh/subrandr',
        commit,
        'Crates resolved by cargo-about from subrandr\'s Cargo.lock for x86_64-pc-windows-gnullvm.',
        [('crates', crates)],
    )


def rust_std_section(paths, build, workspace):
    match = re.search(r'\(([0-9a-f]+) \d{4}-\d{2}-\d{2}\)', build['toolchain']['rustc'])

    if not match:
        logging.error('Release [{}] records no rustc commit'.format(RELEASE))
        abort()

    commit = fetch_json('https://api.github.com/repos/rust-lang/rust/commits/{}'.format(match.group(1)))['sha']
    directory = workspace / 'rust'
    checkout(
        'https://github.com/rust-lang/rust.git', commit, directory,
        sparse=['/library/', '/COPYRIGHT', '/LICENSE-MIT', '/LICENSE-APACHE'],
    )

    texts = [(name, text_of((directory / name).read_bytes(), name)) for name in ['COPYRIGHT', 'LICENSE-MIT', 'LICENSE-APACHE']]
    texts.append((
        'crates',
        cargo_about(paths, directory, STD_ABOUT_CONFIG, 'library/std/Cargo.toml', STD_FEATURES, bootstrap=True),
    ))

    logging.info('Collected the Rust standard library and its crates')

    return section(
        'Rust standard library',
        'MIT OR Apache-2.0, crates as listed below',
        'https://github.com/rust-lang/rust',
        commit,
        'Built by {}; crates resolved by cargo-about for std with [{}] on x86_64-pc-windows-gnullvm.'.format(
            build['toolchain']['rustc'], STD_FEATURES
        ),
        texts,
    )


def header(build):
    archive = next((name for name in build['archives'] if name.startswith('mpv-dev-')), None)

    if not archive:
        logging.error('Release [{}] lists no mpv-dev archive'.format(RELEASE))
        abort()

    return '\n'.join([
        'Third-party notices for libmpv-2.dll in the Shadowmask Windows client',
        '',
        'Generated, do not edit. Regenerate with:',
        '',
        '    ' + REFRESH_CMD,
        '',
        'libmpv-2.dll is {} (SHA-256 {}) from {} release {}:'.format(
            archive, build['archives'][archive], BUILDER, RELEASE
        ),
        'mpv {} and FFmpeg {}. Everything below is linked into that one DLL.'.format(build['mpv'], build['ffmpeg']),
        '',
    ])


def notices(paths):
    build = fetch_json(BUILD_URL)

    if build.get('architecture') != 'x86_64':
        logging.error('[{}] describes [{}], not x86_64'.format(BUILD_URL, build.get('architecture')))
        abort()

    started = build_started(build)
    sections = [header(build)]
    with tempfile.TemporaryDirectory() as directory:
        workspace = Path(directory)
        sections += [component_section(component, build, started, workspace) for component in COMPONENTS]
        sections.append(subrandr_section(paths, build, workspace))
        sections.append(rust_std_section(paths, build, workspace))

    return '\n'.join(sections).rstrip('\n') + '\n'


def refresh_notices(paths, content):
    paths.target.write_text(content, encoding='utf-8')
    logging.info('Refreshed [{}]'.format(TARGET))


def verify_notices(paths, content):
    if not paths.target.is_file() or paths.target.read_text(encoding='utf-8') != content:
        logging.error('[{}] is missing or outdated; refresh with: {}'.format(TARGET, REFRESH_CMD))
        abort()

    logging.info('Verified [{}]'.format(TARGET))


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
        help='Verify that the committed notices match a fresh collection'
    )

    args = parser.parse_args()

    logging.basicConfig(
        format='[%(asctime)-15s] [%(levelname)s] [%(name)-5s]: %(message)s',
        level=logging.getLevelName(logging.DEBUG if args.verbose else logging.INFO)
    )

    os.chdir(paths.repo)

    content = notices(paths)

    if args.verify:
        verify_notices(paths=paths, content=content)
    else:
        refresh_notices(paths=paths, content=content)


if __name__ == '__main__':
    main()
