import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/l10n/strings.dart';

void main() {
  test('an episode code pads both numbers and drops a missing season', () {
    expect(Strings.episodeCode(2, 3), 'S02E03');
    expect(Strings.episodeCode(0, 12), 'S00E12');
    expect(Strings.episodeCode(null, 3), 'E03');
  });

  test('an episode line joins the code and the title with a dot', () {
    expect(Strings.episodeLine(2, 2, 'Earth'), 'S02E02 · Earth');
    expect(
      Strings.episodeLine(2, 2, 'Earth - Part 1'),
      'S02E02 · Earth - Part 1',
    );
    expect(Strings.episodeLine(null, 2, 'Earth'), 'E02 · Earth');
  });

  test(
    'a missing title, or one that only repeats the number, leaves the code',
    () {
      expect(Strings.episodeLine(2, 2, ''), 'S02E02');
      expect(Strings.episodeLine(2, 2, '  '), 'S02E02');
      expect(Strings.episodeLine(2, 2, 'Episode 2'), 'S02E02');
      expect(Strings.episodeLine(2, 2, 'Episode 12'), 'S02E02 · Episode 12');
    },
  );

  test('a series line puts the series first and survives a missing one', () {
    expect(
      Strings.seriesEpisodeLine('Skyline', 2, 2, 'Earth'),
      'Skyline · S02E02 · Earth',
    );
    expect(Strings.seriesEpisodeLine(null, 2, 2, 'Earth'), 'S02E02 · Earth');
    expect(Strings.seriesEpisodeLine(' ', 2, 2, ''), 'S02E02');
  });

  test('a purge names only what it removed, and who lost their data', () {
    expect(
      Strings.toastLibraryPurged(
        movies: 1,
        series: 2,
        seasons: 1,
        episodes: 32,
        subtitles: 1,
        wipedUsers: 1,
      ),
      'Purged: 1 movie, 2 series, 1 season, 32 episodes, 1 subtitle. '
      'User data wiped for 1 user.',
    );
    expect(
      Strings.toastLibraryPurged(
        movies: 0,
        series: 0,
        seasons: 0,
        episodes: 0,
        subtitles: 0,
        wipedUsers: 0,
      ),
      'Purged: nothing to remove.',
    );
    expect(
      Strings.toastLibraryPurged(
        movies: 2,
        series: 0,
        seasons: 0,
        episodes: 0,
        subtitles: 3,
        wipedUsers: 4,
      ),
      'Purged: 2 movies, 3 subtitles. User data wiped for 4 users.',
    );
  });

  test('a clean names only what it removed, and who lost their data', () {
    expect(
      Strings.toastCleaned(
        versions: 1,
        movies: 1,
        series: 1,
        seasons: 2,
        episodes: 1,
        wipedUsers: 0,
      ),
      'Cleaned: 1 version, 1 movie, 1 series, 2 seasons, 1 episode.',
    );
    expect(
      Strings.toastCleaned(
        versions: 0,
        movies: 0,
        series: 0,
        seasons: 0,
        episodes: 0,
        wipedUsers: 0,
      ),
      'Cleaned: nothing to remove.',
    );
    expect(
      Strings.toastCleaned(
        versions: 7,
        movies: 0,
        series: 0,
        seasons: 0,
        episodes: 0,
        wipedUsers: 3,
      ),
      'Cleaned: 7 versions. User data wiped for 3 users.',
    );
  });

  test('a cancel reports each count', () {
    expect(
      Strings.toastLibraryJobsCancelled(
        cancelled: 12,
        signalled: 1,
        running: 0,
      ),
      'Cancelled 12, stopped 1, 0 still running.',
    );
  });
}
