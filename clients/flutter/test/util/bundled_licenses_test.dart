import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/bundled_licenses.dart';

List<String> _libraries(BundlePlatform platform) => <String>[
  for (final BundledLicense license in bundledLicensesFor(platform))
    ...license.libraries,
];

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('macOS and iOS credit the frameworks the darwin build bundles', () {
    for (final BundlePlatform platform in <BundlePlatform>[
      BundlePlatform.macos,
      BundlePlatform.ios,
    ]) {
      expect(_libraries(platform).toSet(), <String>{
        'Roboto',
        'mpv',
        'GNU FriBidi',
        'FFmpeg',
        'dav1d',
        'libass',
        'FreeType',
        'HarfBuzz',
        'Mbed TLS',
        'libpng',
        'uchardet',
        'libxml2',
      }, reason: 'both flavours carry the same set, per CREDITS.md');
    }
  });

  test('Android drops libpng and uchardet and adds zlib', () {
    final Set<String> android = _libraries(BundlePlatform.android).toSet();
    final Set<String> darwin = _libraries(BundlePlatform.macos).toSet();

    expect(darwin.difference(android), <String>{'libpng', 'uchardet'});
    expect(android.difference(darwin), <String>{'zlib'});
  });

  test('Linux credits Roboto and glibc; libmpv and GTK come from the host', () {
    expect(_libraries(BundlePlatform.linux), <String>[
      'Roboto',
      'GNU C Library (glibc)',
    ]);
  });

  test('every build credits the Roboto files it carries in its assets', () {
    for (final BundlePlatform platform in BundlePlatform.values) {
      expect(
        bundledLicensesFor(platform).map((BundledLicense l) => l.asset),
        contains('Roboto-OFL.txt'),
        reason: '$platform ships assets/fonts/',
      );
    }
    expect(_libraries(BundlePlatform.web), <String>['Roboto']);
    expect(_libraries(BundlePlatform.other), <String>['Roboto']);
  });

  test('FFmpeg is credited under both LGPL-3.0 and GPL-3.0', () {
    final List<String> assets = <String>[
      for (final BundledLicense license in bundledLicensesFor(
        BundlePlatform.macos,
      ))
        if (license.libraries.contains('FFmpeg')) license.asset,
    ];

    expect(assets, <String>[
      'LGPL-3.0.txt',
      'GPL-3.0.txt',
    ], reason: 'LGPL-3.0 incorporates GPL-3.0 by reference');
  });

  test(
    'every licence text named by the tables is in the asset bundle',
    () async {
      final Set<String> paths = <String>{
        for (final BundlePlatform platform in BundlePlatform.values)
          for (final BundledLicense license in bundledLicensesFor(platform))
            license.path,
      };

      expect(paths, isNotEmpty);

      for (final String path in paths) {
        final String text = await rootBundle.loadString(path);
        expect(text.trim(), isNotEmpty, reason: '$path is empty');
      }
    },
  );

  test('entries carry the libraries and the text of their licence', () async {
    final List<LicenseEntry> entries = await bundledLicenseEntries(
      BundlePlatform.linux,
      rootBundle,
    ).toList();

    String text(LicenseEntry entry) =>
        entry.paragraphs.map((LicenseParagraph p) => p.text).join(' ');

    expect(entries, hasLength(2));
    expect(entries.first.packages, <String>['Roboto']);
    expect(text(entries.first), contains('SIL OPEN FONT LICENSE'));
    expect(entries.last.packages, <String>['GNU C Library (glibc)']);
    expect(entries.last, isA<LicenseEntryWithLineBreaks>());
    expect(text(entries.last), contains('GNU LESSER GENERAL PUBLIC LICENSE'));
  });

  test('the web build credits only the font it carries', () async {
    final List<LicenseEntry> entries = await bundledLicenseEntries(
      BundlePlatform.web,
      rootBundle,
    ).toList();

    expect(entries.single.packages, <String>['Roboto']);
  });
}
