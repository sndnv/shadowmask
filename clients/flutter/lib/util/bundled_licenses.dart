import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

enum BundlePlatform { macos, ios, android, linux, windows, web, other }

@immutable
class BundledLicense {
  const BundledLicense({required this.asset, required this.libraries});

  final String asset;
  final List<String> libraries;

  String get path => 'assets/licenses/$asset';
}

const BundledLicense _roboto = BundledLicense(
  asset: 'Roboto-OFL.txt',
  libraries: <String>['Roboto'],
);

const List<BundledLicense> _darwin = <BundledLicense>[
  _roboto,
  BundledLicense(
    asset: 'LGPL-2.1.txt',
    libraries: <String>['mpv', 'GNU FriBidi'],
  ),
  BundledLicense(asset: 'LGPL-3.0.txt', libraries: <String>['FFmpeg']),
  BundledLicense(asset: 'GPL-3.0.txt', libraries: <String>['FFmpeg']),
  BundledLicense(asset: 'dav1d.LICENSE.txt', libraries: <String>['dav1d']),
  BundledLicense(asset: 'libass.LICENSE.txt', libraries: <String>['libass']),
  BundledLicense(
    asset: 'FreeType.LICENSE.txt',
    libraries: <String>['FreeType'],
  ),
  BundledLicense(
    asset: 'HarfBuzz.LICENSE.txt',
    libraries: <String>['HarfBuzz'],
  ),
  BundledLicense(asset: 'MbedTLS.LICENSE.txt', libraries: <String>['Mbed TLS']),
  BundledLicense(asset: 'libpng.LICENSE.txt', libraries: <String>['libpng']),
  BundledLicense(
    asset: 'uchardet.LICENSE.txt',
    libraries: <String>['uchardet'],
  ),
  BundledLicense(asset: 'libxml2.LICENSE.txt', libraries: <String>['libxml2']),
];

const List<BundledLicense> _android = <BundledLicense>[
  _roboto,
  BundledLicense(
    asset: 'LGPL-2.1.txt',
    libraries: <String>['mpv', 'GNU FriBidi'],
  ),
  BundledLicense(asset: 'LGPL-3.0.txt', libraries: <String>['FFmpeg']),
  BundledLicense(asset: 'GPL-3.0.txt', libraries: <String>['FFmpeg']),
  BundledLicense(asset: 'dav1d.LICENSE.txt', libraries: <String>['dav1d']),
  BundledLicense(asset: 'libass.LICENSE.txt', libraries: <String>['libass']),
  BundledLicense(
    asset: 'FreeType.LICENSE.txt',
    libraries: <String>['FreeType'],
  ),
  BundledLicense(
    asset: 'HarfBuzz.LICENSE.txt',
    libraries: <String>['HarfBuzz'],
  ),
  BundledLicense(asset: 'MbedTLS.LICENSE.txt', libraries: <String>['Mbed TLS']),
  BundledLicense(asset: 'libxml2.LICENSE.txt', libraries: <String>['libxml2']),
  BundledLicense(asset: 'zlib.LICENSE.txt', libraries: <String>['zlib']),
];

const List<BundledLicense> _linux = <BundledLicense>[
  _roboto,
  BundledLicense(
    asset: 'LGPL-2.1.txt',
    libraries: <String>['GNU C Library (glibc)'],
  ),
];

const List<BundledLicense> _windows = <BundledLicense>[
  _roboto,
  BundledLicense(
    asset: 'libmpv-windows.NOTICES.txt',
    libraries: <String>[
      'mpv',
      'FFmpeg',
      'dav1d',
      'libvpx',
      'uavs3d',
      'libjxl',
      'Highway',
      'Brotli',
      'libopenmpt',
      'libogg',
      'libvorbis',
      'libmodplug',
      'libsoxr',
      'libplacebo',
      'shaderc',
      'glslang',
      'SPIRV-Tools',
      'SPIRV-Headers',
      'SPIRV-Cross',
      'Vulkan-Headers',
      'Little CMS',
      'zimg',
      'graphengine',
      'libass',
      'GNU FriBidi',
      'HarfBuzz',
      'FreeType',
      'Fontconfig',
      'libunibreak',
      'uchardet',
      'GNU libiconv',
      'libva',
      'Intel VPL',
      'AMF',
      'nv-codec-headers',
      'xxHash',
      'VapourSynth headers',
      'OpenSSL',
      'curl',
      'nghttp2',
      'nghttp3',
      'ngtcp2',
      'c-ares',
      'libpsl',
      'Public Suffix List',
      'libssh',
      'SRT',
      'libxml2',
      'libarchive',
      'libjpeg-turbo',
      'libpng',
      'zlib-ng',
      'Zstandard',
      'XZ Utils (liblzma)',
      'bzip2',
      'MinGW-w64 runtime and winpthreads',
      'LLVM libc++, libc++abi and libunwind',
      'C++/WinRT',
      'subrandr',
      'Rust standard library',
    ],
  ),
];

List<BundledLicense> bundledLicensesFor(BundlePlatform platform) {
  switch (platform) {
    case BundlePlatform.macos:
    case BundlePlatform.ios:
      return _darwin;
    case BundlePlatform.android:
      return _android;
    case BundlePlatform.linux:
      return _linux;
    case BundlePlatform.windows:
      return _windows;
    case BundlePlatform.web:
    case BundlePlatform.other:
      return const <BundledLicense>[_roboto];
  }
}

Stream<LicenseEntry> bundledLicenseEntries(
  BundlePlatform platform,
  AssetBundle bundle,
) async* {
  for (final BundledLicense license in bundledLicensesFor(platform)) {
    yield LicenseEntryWithLineBreaks(
      license.libraries,
      await bundle.loadString(license.path),
    );
  }
}
