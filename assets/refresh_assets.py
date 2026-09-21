#!/usr/bin/env python3

import argparse
import logging
import os
import subprocess
import sys
from pathlib import Path

DESCRIPTION = 'Refresh shared assets used by client subprojects'

FLUTTER_MACOS_APPICON = 'macos/Runner/Assets.xcassets/AppIcon.appiconset'
FLUTTER_IOS_APPICON = 'ios/Runner/Assets.xcassets/AppIcon.appiconset'
FLUTTER_ANDROID_RES = 'android/app/src/main/res'
FLUTTER_LICENSES = 'assets/licenses'


class Paths:
    def __init__(self):
        self.current = Path(os.path.realpath(__file__))
        self.repo = self.current.parents[1]
        self.assets = '{}/assets'.format(self.repo)


def abort():
    sys.exit(1)


def source_path(paths, asset):
    return '{}/{}'.format('{}/{}'.format(paths.repo, asset['root']) if 'root' in asset else paths.assets, asset['asset'])


def copy_asset(paths, project_name, asset):
    asset_path = source_path(paths, asset)
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
    asset_path = source_path(paths, asset)
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
            {'asset': 'brand/shadowmask.logo.svg', 'target': 'favicon.svg'},
            {'asset': 'brand/shadowmask.logo-light.svg', 'target': 'logo.svg'},
            {'asset': 'placeholders/poster.svg', 'target': 'placeholder.svg'},
            {'asset': 'placeholders/landscape.svg', 'target': 'placeholder-landscape.svg'},
            {'asset': 'placeholders/person.svg', 'target': 'placeholder-person.svg'},
            {'asset': 'vendor/hls.min.js', 'target': 'vendor/hls.min.js'},
            {'asset': 'vendor/hls.js.LICENSE.txt', 'target': 'vendor/hls.js.LICENSE.txt'},
            {'asset': 'attribution/tmdb.svg', 'target': 'tmdb.svg'},
        ],
        'clients/flutter': [
            {'asset': 'icons/flutter/favicon.png', 'target': 'web/favicon.png'},
            {'asset': 'icons/flutter/Icon-192.png', 'target': 'web/icons/Icon-192.png'},
            {'asset': 'icons/flutter/Icon-512.png', 'target': 'web/icons/Icon-512.png'},
            {'asset': 'icons/flutter/Icon-maskable-192.png', 'target': 'web/icons/Icon-maskable-192.png'},
            {'asset': 'icons/flutter/Icon-maskable-512.png', 'target': 'web/icons/Icon-maskable-512.png'},
            {'asset': 'icons/flutter/app_icon_16.png', 'target': '{}/app_icon_16.png'.format(FLUTTER_MACOS_APPICON)},
            {'asset': 'icons/flutter/app_icon_32.png', 'target': '{}/app_icon_32.png'.format(FLUTTER_MACOS_APPICON)},
            {'asset': 'icons/flutter/app_icon_64.png', 'target': '{}/app_icon_64.png'.format(FLUTTER_MACOS_APPICON)},
            {'asset': 'icons/flutter/app_icon_128.png', 'target': '{}/app_icon_128.png'.format(FLUTTER_MACOS_APPICON)},
            {'asset': 'icons/flutter/app_icon_256.png', 'target': '{}/app_icon_256.png'.format(FLUTTER_MACOS_APPICON)},
            {'asset': 'icons/flutter/app_icon_512.png', 'target': '{}/app_icon_512.png'.format(FLUTTER_MACOS_APPICON)},
            {'asset': 'icons/flutter/app_icon_1024.png', 'target': '{}/app_icon_1024.png'.format(FLUTTER_MACOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_20.png', 'target': '{}/Icon-App-20x20@1x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_40.png', 'target': '{}/Icon-App-20x20@2x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_60.png', 'target': '{}/Icon-App-20x20@3x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_29.png', 'target': '{}/Icon-App-29x29@1x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_58.png', 'target': '{}/Icon-App-29x29@2x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_87.png', 'target': '{}/Icon-App-29x29@3x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_40.png', 'target': '{}/Icon-App-40x40@1x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_80.png', 'target': '{}/Icon-App-40x40@2x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_120.png', 'target': '{}/Icon-App-40x40@3x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_120.png', 'target': '{}/Icon-App-60x60@2x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_180.png', 'target': '{}/Icon-App-60x60@3x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_76.png', 'target': '{}/Icon-App-76x76@1x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_152.png', 'target': '{}/Icon-App-76x76@2x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_167.png', 'target': '{}/Icon-App-83.5x83.5@2x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/ios_icon_1024.png', 'target': '{}/Icon-App-1024x1024@1x.png'.format(FLUTTER_IOS_APPICON)},
            {'asset': 'icons/flutter/android_icon_48.png', 'target': '{}/mipmap-mdpi/ic_launcher.png'.format(FLUTTER_ANDROID_RES)},
            {'asset': 'icons/flutter/android_icon_72.png', 'target': '{}/mipmap-hdpi/ic_launcher.png'.format(FLUTTER_ANDROID_RES)},
            {'asset': 'icons/flutter/android_icon_96.png', 'target': '{}/mipmap-xhdpi/ic_launcher.png'.format(FLUTTER_ANDROID_RES)},
            {'asset': 'icons/flutter/android_icon_144.png', 'target': '{}/mipmap-xxhdpi/ic_launcher.png'.format(FLUTTER_ANDROID_RES)},
            {'asset': 'icons/flutter/android_icon_192.png', 'target': '{}/mipmap-xxxhdpi/ic_launcher.png'.format(FLUTTER_ANDROID_RES)},
            {'asset': 'icons/flutter/android_icon_fg_108.png', 'target': '{}/mipmap-mdpi/ic_launcher_foreground.png'.format(FLUTTER_ANDROID_RES)},
            {'asset': 'icons/flutter/android_icon_fg_162.png', 'target': '{}/mipmap-hdpi/ic_launcher_foreground.png'.format(FLUTTER_ANDROID_RES)},
            {'asset': 'icons/flutter/android_icon_fg_216.png', 'target': '{}/mipmap-xhdpi/ic_launcher_foreground.png'.format(FLUTTER_ANDROID_RES)},
            {'asset': 'icons/flutter/android_icon_fg_324.png', 'target': '{}/mipmap-xxhdpi/ic_launcher_foreground.png'.format(FLUTTER_ANDROID_RES)},
            {'asset': 'icons/flutter/android_icon_fg_432.png', 'target': '{}/mipmap-xxxhdpi/ic_launcher_foreground.png'.format(FLUTTER_ANDROID_RES)},
            {'asset': 'vendor/hls.min.js', 'target': 'web/hls.min.js'},
            {'asset': 'vendor/hls.js.LICENSE.txt', 'target': 'web/hls.js.LICENSE.txt'},
            {'asset': 'icons/flutter/tmdb-logo.png', 'target': 'assets/attribution/tmdb-logo.png'},
            {'root': 'licenses', 'asset': 'dav1d.LICENSE.txt', 'target': '{}/dav1d.LICENSE.txt'.format(FLUTTER_LICENSES)},
            {'root': 'licenses', 'asset': 'FreeType.LICENSE.txt', 'target': '{}/FreeType.LICENSE.txt'.format(FLUTTER_LICENSES)},
            {'root': 'licenses', 'asset': 'GPL-3.0.txt', 'target': '{}/GPL-3.0.txt'.format(FLUTTER_LICENSES)},
            {'root': 'licenses', 'asset': 'HarfBuzz.LICENSE.txt', 'target': '{}/HarfBuzz.LICENSE.txt'.format(FLUTTER_LICENSES)},
            {'root': 'licenses', 'asset': 'LGPL-2.1.txt', 'target': '{}/LGPL-2.1.txt'.format(FLUTTER_LICENSES)},
            {'root': 'licenses', 'asset': 'LGPL-3.0.txt', 'target': '{}/LGPL-3.0.txt'.format(FLUTTER_LICENSES)},
            {'root': 'licenses', 'asset': 'libass.LICENSE.txt', 'target': '{}/libass.LICENSE.txt'.format(FLUTTER_LICENSES)},
            {'root': 'licenses', 'asset': 'libpng.LICENSE.txt', 'target': '{}/libpng.LICENSE.txt'.format(FLUTTER_LICENSES)},
            {'root': 'licenses', 'asset': 'libxml2.LICENSE.txt', 'target': '{}/libxml2.LICENSE.txt'.format(FLUTTER_LICENSES)},
            {'root': 'licenses', 'asset': 'MbedTLS.LICENSE.txt', 'target': '{}/MbedTLS.LICENSE.txt'.format(FLUTTER_LICENSES)},
            {'root': 'licenses', 'asset': 'uchardet.LICENSE.txt', 'target': '{}/uchardet.LICENSE.txt'.format(FLUTTER_LICENSES)},
            {'root': 'licenses', 'asset': 'zlib.LICENSE.txt', 'target': '{}/zlib.LICENSE.txt'.format(FLUTTER_LICENSES)},
        ],
        'clients/roku': [
            {'asset': 'fonts/Roboto-Regular.ttf', 'target': 'fonts/Roboto-Regular.ttf'},
            {'asset': 'fonts/Roboto-Medium.ttf', 'target': 'fonts/Roboto-Medium.ttf'},
            {'asset': 'fonts/Roboto-OFL.txt', 'target': 'fonts/Roboto-OFL.txt'},
            {'asset': 'icons/roku/channel-poster-hd.png', 'target': 'images/channel-poster-hd.png'},
            {'asset': 'icons/roku/channel-poster-fhd.png', 'target': 'images/channel-poster-fhd.png'},
            {'asset': 'icons/roku/splash-sd.png', 'target': 'images/splash-sd.png'},
            {'asset': 'icons/roku/splash-hd.png', 'target': 'images/splash-hd.png'},
            {'asset': 'icons/roku/splash-fhd.png', 'target': 'images/splash-fhd.png'},
            {'asset': 'icons/roku/brand-mark.png', 'target': 'images/brand-mark.png'},
            {'asset': 'icons/roku/tmdb-logo.png', 'target': 'images/tmdb-logo.png'},
            {'asset': 'icons/roku/art-movie.png', 'target': 'images/art-movie.png'},
            {'asset': 'icons/roku/art-landscape.png', 'target': 'images/art-landscape.png'},
            {'asset': 'icons/roku/art-person.png', 'target': 'images/art-person.png'},
            {'asset': 'icons/roku/watched-disc.png', 'target': 'images/watched-disc.png'},
            {'asset': 'icons/roku/watched-ring.png', 'target': 'images/watched-ring.png'},
            {'asset': 'icons/roku/hex-texture.png', 'target': 'images/hex-texture.png'},
            {'asset': 'icons/roku/chevron-left.png', 'target': 'images/chevron-left.png'},
            {'asset': 'icons/roku/chevron-right.png', 'target': 'images/chevron-right.png'},
            {'asset': 'icons/roku/chevron-up.png', 'target': 'images/chevron-up.png'},
            {'asset': 'icons/roku/chevron-down.png', 'target': 'images/chevron-down.png'},
            {'asset': 'icons/roku/icon-check.png', 'target': 'images/icon-check.png'},
            {'asset': 'icons/roku/icon-close.png', 'target': 'images/icon-close.png'},
            {'asset': 'icons/roku/icon-play.png', 'target': 'images/icon-play.png'},
            {'asset': 'icons/roku/icon-pause.png', 'target': 'images/icon-pause.png'},
            {'asset': 'icons/roku/icon-play-large.png', 'target': 'images/icon-play-large.png'},
            {'asset': 'icons/roku/icon-replay.png', 'target': 'images/icon-replay.png'},
            {'asset': 'icons/roku/icon-replay-large.png', 'target': 'images/icon-replay-large.png'},
            {'asset': 'icons/roku/icon-previous.png', 'target': 'images/icon-previous.png'},
            {'asset': 'icons/roku/icon-next.png', 'target': 'images/icon-next.png'},
            {'asset': 'icons/roku/icon-subtitles.png', 'target': 'images/icon-subtitles.png'},
            {'asset': 'icons/roku/icon-audio.png', 'target': 'images/icon-audio.png'},
            {'asset': 'icons/roku/icon-quality.png', 'target': 'images/icon-quality.png'},
            {'asset': 'icons/roku/icon-settings.png', 'target': 'images/icon-settings.png'},
            {'asset': 'icons/roku/icon-search.png', 'target': 'images/icon-search.png'},
            {'asset': 'icons/roku/icon-bookmark.png', 'target': 'images/icon-bookmark.png'},
            {'asset': 'icons/roku/icon-bookmark-on.png', 'target': 'images/icon-bookmark-on.png'},
            {'asset': 'icons/roku/icon-heart.png', 'target': 'images/icon-heart.png'},
            {'asset': 'icons/roku/icon-heart-on.png', 'target': 'images/icon-heart-on.png'},
            {'asset': 'icons/roku/icon-shuffle.png', 'target': 'images/icon-shuffle.png'},
            {'asset': 'icons/roku/icon-trash.png', 'target': 'images/icon-trash.png'},
            {'asset': 'icons/roku/icon-sort.png', 'target': 'images/icon-sort.png'},
            {'asset': 'icons/roku/icon-filter.png', 'target': 'images/icon-filter.png'},
            {'asset': 'icons/roku/icon-library.png', 'target': 'images/icon-library.png'},
            {'asset': 'icons/roku/chip-cap-left.png', 'target': 'images/chip-cap-left.png'},
            {'asset': 'icons/roku/chip-cap-right.png', 'target': 'images/chip-cap-right.png'},
            {'asset': 'icons/roku/chip-cap-left-line.png', 'target': 'images/chip-cap-left-line.png'},
            {'asset': 'icons/roku/chip-cap-right-line.png', 'target': 'images/chip-cap-right-line.png'},
            {'asset': 'icons/roku/button-fill.9.png', 'target': 'images/button-fill.9.png'},
            {'asset': 'icons/roku/button-line.9.png', 'target': 'images/button-line.9.png'},
            {'asset': 'icons/roku/disc.png', 'target': 'images/disc.png'},
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
