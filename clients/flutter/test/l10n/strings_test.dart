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
}
