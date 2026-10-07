import 'package:flutter/services.dart';

import 'package:shadowmask/util/bundled_licenses.dart';

const String kBundledSansFamily = 'Roboto';

const List<String> kBundledSansAssets = <String>[
  'assets/fonts/Roboto-Regular.ttf',
  'assets/fonts/Roboto-Medium.ttf',
  'assets/fonts/Roboto-Bold.ttf',
];

bool loadsBundledFonts(BundlePlatform platform) =>
    platform == BundlePlatform.linux;

Future<void> loadBundledFonts(
  BundlePlatform platform,
  AssetBundle bundle, {
  FontLoader Function(String family) loaderFor = FontLoader.new,
}) async {
  if (!loadsBundledFonts(platform)) {
    return;
  }
  try {
    final List<ByteData> faces = <ByteData>[
      for (final String asset in kBundledSansAssets) await bundle.load(asset),
    ];
    final FontLoader loader = loaderFor(kBundledSansFamily);
    for (final ByteData face in faces) {
      loader.addFont(Future<ByteData>.value(face));
    }
    await loader.load();
  } catch (_) {}
}
