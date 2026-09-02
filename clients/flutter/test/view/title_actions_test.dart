import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/view/title_actions.dart';

Version _version({bool available = true}) => Version(
  id: 'v1',
  title: const TitleRef(type: TitleKind.movie, id: 'm1'),
  libraryId: 'lib1',
  quality: Quality.fhd,
  container: 'mkv',
  available: available,
);

void main() {
  test('a title whose files are all present is relinkable', () {
    expect(relinkBlockForVersions(<Version>[_version()]), isNull);
  });

  test('one missing file blocks the relink and is counted', () {
    expect(
      relinkBlockForVersions(<Version>[_version(), _version(available: false)]),
      Strings.relinkBlockedMovie(1),
    );
  });

  test('every missing file is counted, not just the first', () {
    expect(
      relinkBlockForVersions(<Version>[
        _version(available: false),
        _version(available: false),
        _version(),
      ]),
      Strings.relinkBlockedMovie(2),
    );
  });

  test('a series with every episode playable is relinkable', () {
    expect(relinkBlockForSeries(4, 4), isNull);
  });

  test('a series names how many episodes have no file', () {
    expect(relinkBlockForSeries(4, 1), Strings.relinkBlockedSeries(3));
  });

  test('an empty series says there is nothing to relink', () {
    expect(relinkBlockForSeries(0, 0), Strings.relinkBlockedEmptySeries);
  });

  test('a title is only deletable once nothing is left under it', () {
    expect(deleteBlockForVersions(0), isNull);
    expect(deleteBlockForVersions(2), Strings.blockedByVersions(2));
    expect(deleteBlockForEpisodes(0), isNull);
    expect(deleteBlockForEpisodes(3), Strings.blockedByEpisodes(3));
    expect(deleteBlockForSeasons(0), isNull);
    expect(deleteBlockForSeasons(1), Strings.blockedBySeasons(1));
  });

  test('the singular reads as a sentence, not as a count of one', () {
    expect(Strings.blockedByVersions(1), contains('1 remaining version '));
    expect(Strings.blockedByEpisodes(1), contains('1 remaining episode '));
    expect(Strings.relinkBlockedSeries(1), startsWith('One episode'));
  });
}
