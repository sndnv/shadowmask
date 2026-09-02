import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/view/version_order.dart';

Version _version(String id, Quality quality, {int sizeBytes = 0}) => Version(
  id: id,
  title: const TitleRef(type: TitleKind.movie, id: 'm1'),
  libraryId: 'l1',
  quality: quality,
  container: 'mkv',
  sizeBytes: sizeBytes,
);

void main() {
  test('versions order by quality, then size, then id', () {
    final List<Version> ordered = orderedVersions(<Version>[
      _version('c', Quality.hd),
      _version('a', Quality.uhd),
      _version('d', Quality.fhd, sizeBytes: 10),
      _version('b', Quality.fhd, sizeBytes: 20),
    ]);

    expect(ordered.map((Version v) => v.id).toList(), <String>[
      'a',
      'b',
      'd',
      'c',
    ]);
  });

  test('numbers are one based positions in the shared order', () {
    expect(versionNumberLabel(0), '1');
    expect(versionNumberLabel(3), '4');
  });
}
