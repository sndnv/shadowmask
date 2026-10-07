import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/bundled_fonts.dart';
import 'package:shadowmask/util/bundled_licenses.dart';

class _RecordingLoader extends FontLoader {
  _RecordingLoader(super.family);

  final List<Future<ByteData>> fonts = <Future<ByteData>>[];
  bool loaded = false;

  @override
  void addFont(Future<ByteData> bytes) => fonts.add(bytes);

  @override
  Future<void> load() async => loaded = true;
}

class _MissingBundle extends CachingAssetBundle {
  @override
  Future<ByteData> load(String key) async =>
      throw FlutterError('Unable to load asset: "$key".');
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<List<_RecordingLoader>> loadFor(BundlePlatform platform) async {
    final List<_RecordingLoader> created = <_RecordingLoader>[];
    await loadBundledFonts(
      platform,
      rootBundle,
      loaderFor: (String family) {
        final _RecordingLoader loader = _RecordingLoader(family);
        created.add(loader);
        return loader;
      },
    );
    return created;
  }

  test(
    'Linux registers the bundled Roboto under the name it asks for',
    () async {
      final List<_RecordingLoader> created = await loadFor(
        BundlePlatform.linux,
      );

      expect(created, hasLength(1));
      expect(created.single.family, 'Roboto');
      expect(created.single.fonts, hasLength(kBundledSansAssets.length));
      expect(created.single.loaded, isTrue);
    },
  );

  test('a font that cannot be read leaves the app to start', () async {
    await expectLater(
      loadBundledFonts(BundlePlatform.linux, _MissingBundle()),
      completes,
    );
  });

  test('every other platform keeps its own system font', () async {
    for (final BundlePlatform platform in BundlePlatform.values) {
      if (platform == BundlePlatform.linux) {
        continue;
      }
      expect(await loadFor(platform), isEmpty, reason: '$platform');
    }
  });

  test('regular, medium and bold are all in the asset bundle', () async {
    for (final String asset in kBundledSansAssets) {
      final ByteData bytes = await rootBundle.load(asset);
      expect(bytes.lengthInBytes, greaterThan(100000), reason: asset);
    }
  });
}
