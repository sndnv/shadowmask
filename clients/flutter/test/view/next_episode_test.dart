import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/season.dart';
import 'package:shadowmask/view/next_episode.dart';
import 'package:shared_preferences/shared_preferences.dart';

Season _season(String id, int number) =>
    Season(id: id, seriesId: 's1', number: number);

Episode _episode(String id, int number) =>
    Episode(id: id, seasonId: 'se1', number: number, title: 'Episode $number');

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('the next season is the first that is not fully watched', () {
    final List<Season> seasons = <Season>[
      _season('se2', 2),
      _season('se1', 1),
      _season('se3', 3),
    ];

    expect(nextSeason(seasons, <String>{'se1'})?.id, 'se2');
    expect(nextSeason(seasons, <String>{})?.id, 'se1');
    expect(nextSeason(seasons, <String>{'se1', 'se2', 'se3'})?.id, 'se1');
    expect(nextSeason(const <Season>[], <String>{}), isNull);
  });

  test('specials never win while the series has a regular season', () {
    final List<Season> seasons = <Season>[
      _season('sp', 0),
      _season('se1', 1),
      _season('se2', 2),
    ];

    expect(nextSeason(seasons, <String>{})?.id, 'se1');
    expect(nextSeason(seasons, <String>{'se1'})?.id, 'se2');
    expect(nextSeason(seasons, <String>{'se1', 'se2'})?.id, 'se1');
  });

  test('a series with only specials plays its specials', () {
    expect(nextSeason(<Season>[_season('sp', 0)], <String>{})?.id, 'sp');
    expect(nextSeason(<Season>[_season('sp', 0)], <String>{'sp'})?.id, 'sp');
  });

  test('the action says Resume only when the episode has progress', () {
    expect(nextEpisodeAction(null), Strings.play);
    expect(
      nextEpisodeAction((
        episode: _episode('e1', 1),
        season: 1,
        progressPercent: 0,
      )),
      Strings.play,
    );
    expect(
      nextEpisodeAction((
        episode: _episode('e3', 3),
        season: 2,
        progressPercent: 40,
      )),
      Strings.resumeAction,
    );
  });

  test('the caption names the episode, after its season where asked', () {
    final NextEpisode titled = (
      episode: const Episode(
        id: 'e3',
        seasonId: 'se2',
        number: 3,
        title: 'The Hollow Men',
      ),
      season: 2,
      progressPercent: 40,
    );
    final NextEpisode untitled = (
      episode: const Episode(id: 'e4', seasonId: 'se2', number: 4, title: ' '),
      season: 2,
      progressPercent: 0,
    );
    final NextEpisode numbered = (
      episode: _episode('e5', 5),
      season: 2,
      progressPercent: 0,
    );

    expect(
      nextEpisodeName((
        episode: const Episode(
          id: 'e2',
          seasonId: 'se2',
          number: 2,
          title: 'Earth - Part 1',
        ),
        season: 2,
        progressPercent: 0,
      ), withSeason: true),
      'S02E02 · Earth - Part 1',
      reason: 'a dash in a title is never mistaken for the separator',
    );
    expect(
      nextEpisodeCaption(titled, withSeason: true),
      'Up next: S02E03 · The Hollow Men',
    );
    expect(
      nextEpisodeCaption(titled, withSeason: false),
      'Up next: E03 · The Hollow Men',
    );
    expect(nextEpisodeCaption(untitled, withSeason: true), 'Up next: S02E04');
    expect(
      nextEpisodeCaption(numbered, withSeason: true),
      'Up next: S02E05',
      reason: 'a title that only repeats the number adds nothing',
    );
    expect(
      nextEpisodeSpoken(titled, withSeason: true),
      'Up next: Season 2, Episode 3, The Hollow Men',
    );
    expect(
      nextEpisodeSpoken(untitled, withSeason: false),
      'Up next: Episode 4',
    );
  });

  test('the next episode in a season carries its progress', () {
    final NextEpisode? next = nextInSeason(
      2,
      <Episode>[_episode('e1', 1), _episode('e2', 2), _episode('e3', 3)],
      <String>{'e1'},
      <String, int>{'e2': 55},
    );

    expect(next?.episode.id, 'e2');
    expect(next?.season, 2);
    expect(next?.progressPercent, 55);
    expect(
      nextInSeason(1, const <Episode>[], <String>{}, <String, int>{}),
      isNull,
    );
  });

  test('the next episode is the first unwatched in episode order', () {
    final List<Episode> episodes = <Episode>[
      _episode('e3', 3),
      _episode('e1', 1),
      _episode('e2', 2),
    ];

    expect(firstUnwatched(episodes, <String>{'e1'})?.id, 'e2');
    expect(firstUnwatched(episodes, <String>{})?.id, 'e1');
    expect(firstUnwatched(episodes, <String>{'e1', 'e2', 'e3'})?.id, 'e1');
    expect(firstUnwatched(const <Episode>[], <String>{}), isNull);
  });

  test(
    'resolving reads the first open season and its episode states',
    () async {
      final List<String> paths = <String>[];
      final CatalogApi catalog = CatalogApi(
        ApiClient(
          baseUrl: 'http://test',
          httpClient: MockClient((http.Request req) async {
            paths.add(req.url.path);
            if (req.url.path.endsWith('/episodes')) {
              return http.Response(
                jsonEncode(<Map<String, dynamic>>[
                  <String, dynamic>{
                    'id': 'e2',
                    'season_id': 'se2',
                    'number': 2,
                    'title': 'Two',
                  },
                  <String, dynamic>{
                    'id': 'e1',
                    'season_id': 'se2',
                    'number': 1,
                    'title': 'One',
                  },
                ]),
                200,
                headers: <String, String>{'content-type': 'application/json'},
              );
            }
            return http.Response(
              jsonEncode(<Map<String, dynamic>>[
                <String, dynamic>{
                  'title': <String, String>{'type': 'episode', 'id': 'e1'},
                  'watched': true,
                },
                <String, dynamic>{
                  'title': <String, String>{'type': 'episode', 'id': 'e2'},
                  'progress_percent': 30,
                },
              ]),
              200,
              headers: <String, String>{'content-type': 'application/json'},
            );
          }),
        ),
      );

      final NextEpisode? next = await resolveNextEpisode(
        catalog,
        'u1',
        's1',
        <Season>[_season('sp', 0), _season('se1', 1), _season('se2', 2)],
        <String>{'se1'},
      );

      expect(next?.episode.id, 'e2');
      expect(next?.season, 2);
      expect(next?.progressPercent, 30);
      expect(paths.first, '/api/v1/series/s1/seasons/se2/episodes');
      expect(paths.last, '/api/v1/users/u1/state/batch');
    },
  );

  test('a season with no episodes resolves to nothing', () async {
    final CatalogApi catalog = CatalogApi(
      ApiClient(
        baseUrl: 'http://test',
        httpClient: MockClient(
          (http.Request req) async => http.Response(
            '[]',
            200,
            headers: <String, String>{'content-type': 'application/json'},
          ),
        ),
      ),
    );

    expect(
      await resolveNextEpisode(catalog, 'u1', 's1', <Season>[
        _season('se1', 1),
      ], <String>{}),
      isNull,
    );
  });

  test('a failed episode read resolves to nothing', () async {
    final CatalogApi catalog = CatalogApi(
      ApiClient(
        baseUrl: 'http://test',
        httpClient: MockClient(
          (http.Request req) async => http.Response('nope', 500),
        ),
      ),
    );

    expect(
      await resolveNextEpisode(catalog, 'u1', 's1', <Season>[
        _season('se1', 1),
      ], <String>{}),
      isNull,
    );
  });
}
