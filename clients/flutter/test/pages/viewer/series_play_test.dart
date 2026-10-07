import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/episode_play_button.dart';
import 'package:shadowmask/components/poster_play.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/viewer/title_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _season(String id, int number) => <String, dynamic>{
  'id': id,
  'series_id': 's1',
  'number': number,
  'title': number == 0 ? 'Specials' : 'Season $number',
  'artwork': <String, dynamic>{},
};

List<dynamic> _targets(String body, String key) =>
    (jsonDecode(body) as Map<String, dynamic>)[key] as List<dynamic>;

ApiClient _api({
  List<String>? seen,
  Set<String> watchedSeasons = const <String>{},
  String? inProgress,
  Duration lag = Duration.zero,
  List<int> seasons = const <int>[0, 1, 2],
}) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    final String path = req.url.path;
    seen?.add(path);
    if (path == '/api/v1/users/self') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': 'user',
        }),
        200,
      );
    }
    if (path == '/api/v1/series/s1') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 's1',
          'title': 'Skyline',
          'artwork': <String, dynamic>{},
          'genres': <String>[],
          'credits': <dynamic>[],
        }),
        200,
      );
    }
    if (path == '/api/v1/series/s1/seasons') {
      return http.Response(
        jsonEncode(<Map<String, dynamic>>[
          for (final int n in seasons) _season(n == 0 ? 'sp' : 'se$n', n),
        ]),
        200,
      );
    }
    if (path.endsWith('/state/rollup')) {
      return http.Response(
        jsonEncode(<Map<String, dynamic>>[
          for (final dynamic t in _targets(req.body, 'targets'))
            <String, dynamic>{
              'target': t,
              'watched': watchedSeasons.contains(
                (t as Map<String, dynamic>)['id'],
              ),
              'total_episodes': t['id'] == 's1' ? 6 : 2,
              'watched_episodes': t['id'] == 's1'
                  ? 2 * watchedSeasons.length
                  : 0,
            },
        ]),
        200,
      );
    }
    if (path.endsWith('/episodes')) {
      await Future<void>.delayed(lag);
      final String season =
          req.url.pathSegments[req.url.pathSegments.length - 2];
      return http.Response(
        jsonEncode(<Map<String, dynamic>>[
          for (final int n in <int>[1, 2])
            <String, dynamic>{
              'id': '${season}e$n',
              'season_id': season,
              'number': n,
              'title': 'Part $n',
            },
        ]),
        200,
      );
    }
    if (path.endsWith('/state/batch')) {
      return http.Response(
        jsonEncode(<Map<String, dynamic>>[
          for (final dynamic t in _targets(req.body, 'titles'))
            <String, dynamic>{
              'title': t,
              'progress_percent':
                  (t as Map<String, dynamic>)['id'] == inProgress ? 40 : 0,
            },
        ]),
        200,
      );
    }
    if (path.endsWith('/versions')) {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'items': <Map<String, dynamic>>[
            <String, dynamic>{
              'id': 'v1',
              'title': <String, dynamic>{'type': 'episode', 'id': 'x'},
              'library_id': 'lib1',
              'container': 'mkv',
              'quality': 'fhd',
              'available': true,
            },
          ],
          'total': 1,
          'offset': 0,
          'limit': 20,
        }),
        200,
      );
    }
    if (path.endsWith('/continue')) {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'now_playing': <dynamic>[],
          'in_progress': <dynamic>[],
          'next_episodes': <dynamic>[],
        }),
        200,
      );
    }
    return http.Response('[]', 200);
  }),
);

Future<void> _pump(
  WidgetTester tester,
  ApiClient api, {
  TargetPlatform platform = TargetPlatform.macOS,
  Size window = const Size(1600, 1400),
}) async {
  tester.view.physicalSize = window;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark).copyWith(platform: platform),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) => settings.name == null || settings.name == '/'
              ? TitlePage(api: api, kind: 'series', id: 's1')
              : Text('went to ${settings.name}'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Finder _button(String label) => find.descendant(
  of: find.byType(EpisodePlayButton),
  matching: find.text(label),
);

Finder _note(String text) => find.descendant(
  of: find.byType(NextEpisodeNote),
  matching: find.text(text),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the page paints before the episode is known, then names it', (
    WidgetTester tester,
  ) async {
    const Duration lag = Duration(seconds: 2);
    final List<String> seen = <String>[];
    await _pump(tester, _api(seen: seen, lag: lag));

    expect(find.text('Skyline'), findsWidgets);
    expect(_button(Strings.play), findsOneWidget);
    expect(_note('Up next: S01E01 · Part 1'), findsNothing);

    await tester.pump(lag);
    await tester.pumpAndSettle();

    expect(_button(Strings.play), findsOneWidget);
    expect(_note('Up next: S01E01 · Part 1'), findsOneWidget);
    expect(
      tester.getRect(_note('Up next: S01E01 · Part 1')).top,
      greaterThan(tester.getRect(find.byType(EpisodePlayButton)).bottom),
      reason: 'the episode is named under the action row, not in the button',
    );
    expect(seen.where((String p) => p.endsWith('/episodes')), <String>[
      '/api/v1/series/s1/seasons/se1/episodes',
    ]);
    expect(
      seen.where((String p) => p.endsWith('/versions')),
      isEmpty,
      reason: 'the versions are read when the button is pressed',
    );
  });

  testWidgets('the page counts regular seasons and what has been watched', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(watchedSeasons: <String>{'se1'}));

    expect(
      find.text(Strings.countLabel(Strings.seasonsHeading, 2)),
      findsOneWidget,
      reason: 'the Specials card shows but is not counted',
    );
    expect(find.text('Specials'), findsOneWidget);
    expect(find.text(Strings.watchedCount(2, 6)), findsOneWidget);
    expect(find.text('2 episodes'), findsNWidgets(3));
  });

  testWidgets('a Specials-only series shows no season count', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(seasons: <int>[0]));

    expect(find.text(Strings.seasonsHeading), findsOneWidget);
    expect(find.textContaining('${Strings.seasonsHeading} ('), findsNothing);
  });

  testWidgets('specials never win over a regular season', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(watchedSeasons: <String>{'se1'}));

    expect(_note('Up next: S02E01 · Part 1'), findsOneWidget);
  });

  testWidgets('an episode in progress reads Resume', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(watchedSeasons: <String>{'se1'}, inProgress: 'se2e1'),
    );

    expect(_button(Strings.resumeAction), findsOneWidget);
    expect(_note('Up next: S02E01 · Part 1'), findsOneWidget);
  });

  testWidgets('a phone plays the series from its button', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(),
      platform: TargetPlatform.android,
      window: const Size(400, 900),
    );

    final Finder play = _button(Strings.play);
    await tester.ensureVisible(play);
    await tester.pumpAndSettle();
    await tester.tap(play);
    await tester.pumpAndSettle();

    expect(find.text('went to /watch?version=v1'), findsOneWidget);
  });

  testWidgets('the poster plays the episode instead of opening its page', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api());

    await tester.tap(find.byType(PosterPlay));
    await tester.pumpAndSettle();

    expect(find.text('went to /watch?version=v1'), findsOneWidget);
  });
}
