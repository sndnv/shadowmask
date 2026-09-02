import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/season.dart';
import 'package:shadowmask/view/episode_neighbours.dart';

Season _season(String id, int number) =>
    Season(id: id, seriesId: 's1', number: number);

Episode _episode(String id, String seasonId, int number) =>
    Episode(id: id, seasonId: seasonId, number: number, title: 'Ep $number');

final List<Season> _seasons = <Season>[_season('se2', 2), _season('se1', 1)];

final Map<String, List<Episode>> _all = <String, List<Episode>>{
  'se1': <Episode>[_episode('e2', 'se1', 2), _episode('e1', 'se1', 1)],
  'se2': <Episode>[_episode('e3', 'se2', 1), _episode('e4', 'se2', 2)],
};

void main() {
  test('a middle episode walks within its own season', () {
    final EpisodeNeighbours n = neighboursOf(
      _episode('e2', 'se1', 2),
      _seasons,
      _all,
    );

    expect(n.previous?.id, 'e1');
    expect(n.next?.id, 'e3', reason: 'season 1 ends, so next rolls over');
  });

  test('the last episode of a season rolls into the next season', () {
    final EpisodeNeighbours n = neighboursOf(
      _episode('e2', 'se1', 2),
      _seasons,
      _all,
    );

    expect(n.next?.id, 'e3');
    expect(n.next?.seasonId, 'se2');
    expect(n.next?.label, 'S02E01 · Ep 1');
  });

  test('the first episode of a season rolls back into the previous one', () {
    final EpisodeNeighbours n = neighboursOf(
      _episode('e3', 'se2', 1),
      _seasons,
      _all,
    );

    expect(n.previous?.id, 'e2');
    expect(n.previous?.seasonId, 'se1');
    expect(n.previous?.label, 'S01E02 · Ep 2');
    expect(n.next?.id, 'e4');
  });

  test('the very first and very last episodes have one side only', () {
    final EpisodeNeighbours first = neighboursOf(
      _episode('e1', 'se1', 1),
      _seasons,
      _all,
    );
    expect(first.previous, isNull);
    expect(first.next?.id, 'e2');

    final EpisodeNeighbours last = neighboursOf(
      _episode('e4', 'se2', 2),
      _seasons,
      _all,
    );
    expect(last.next, isNull);
    expect(last.previous?.id, 'e3');
  });

  test('an unknown season or episode yields nothing rather than guessing', () {
    expect(neighboursOf(_episode('x', 'gone', 1), _seasons, _all).next, isNull);
    expect(
      neighboursOf(_episode('stray', 'se1', 9), _seasons, _all).previous,
      isNull,
    );
  });

  test('only the seasons an edge actually needs are loaded', () {
    expect(
      seasonsToLoad(_episode('e2', 'se1', 2), _seasons, _all['se1']!),
      <String>['se2'],
      reason: 'at the end of season 1, only season 2 is worth fetching',
    );
    expect(
      seasonsToLoad(_episode('e3', 'se2', 1), _seasons, _all['se2']!),
      <String>['se1'],
    );
    expect(
      seasonsToLoad(_episode('e1', 'se1', 1), _seasons, _all['se1']!),
      isEmpty,
      reason: 'the first episode of the first season has nowhere to look',
    );
  });

  test('seasons step one at a time and stop at the ends', () {
    final SeasonNeighbours first = seasonNeighboursOf('se1', _seasons);
    expect(first.previous, isNull);
    expect(first.next?.id, 'se2');
    expect(first.next?.label, 'Season 2');

    final SeasonNeighbours last = seasonNeighboursOf('se2', _seasons);
    expect(last.previous?.id, 'se1');
    expect(last.next, isNull);
  });

  test('a named season shows its name rather than its number', () {
    final SeasonNeighbours n = seasonNeighboursOf('se1', <Season>[
      _season('se1', 1),
      Season(id: 'se2', seriesId: 's1', number: 2, title: 'The Long Winter'),
    ]);

    expect(n.next?.label, 'The Long Winter');
  });

  test('an unknown season has no neighbours', () {
    final SeasonNeighbours n = seasonNeighboursOf('gone', _seasons);
    expect(n.previous, isNull);
    expect(n.next, isNull);
  });

  test('a neighbouring season with no episodes is not offered', () {
    final EpisodeNeighbours n = neighboursOf(
      _episode('e2', 'se1', 2),
      _seasons,
      <String, List<Episode>>{'se1': _all['se1']!, 'se2': const <Episode>[]},
    );

    expect(n.next, isNull);
    expect(n.previous?.id, 'e1');
  });
}
