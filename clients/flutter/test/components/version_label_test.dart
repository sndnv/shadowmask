import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/version_label.dart';
import 'package:shadowmask/model/catalog/version.dart';

Version _version({int durationMs = 0, int sizeBytes = 0}) =>
    Version.fromJson(<String, dynamic>{
      'id': 'v1',
      'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      'library_id': 'l',
      'quality': 'uhd',
      'container': 'mkv',
      'duration_ms': durationMs,
      'size_bytes': sizeBytes,
    });

void main() {
  test(
    'a version reads as its number, length, quality, container and size',
    () {
      expect(
        versionLabelText(
          _version(durationMs: 7800000, sizeBytes: 5368709120),
          '1',
        ),
        '1 · 2h 10m · UHD · mkv · 5.0 GB',
      );
    },
  );

  test('an unknown length or size is left out', () {
    expect(versionLabelText(_version(), '2'), '2 · UHD · mkv');
  });
}
