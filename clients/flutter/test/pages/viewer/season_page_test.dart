import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/components/random_button.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/viewer/season_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _season() => <String, dynamic>{
  'id': 'se1',
  'series_id': 's1',
  'series_title': 'The Show',
  'series_artwork': <String, dynamic>{
    'posters': <Map<String, dynamic>>[
      <String, dynamic>{
        'base': '/images/s1-poster',
        'widths': <int>[180],
      },
    ],
  },
  'number': 0,
  'title': 'Specials',
  'added_at': '2026-08-17T09:00:00Z',
  'updated_at': '2026-08-17T09:00:00Z',
  'artwork': <String, dynamic>{},
};

Future<void> _pump(WidgetTester tester, ApiClient api) async {
  tester.view.physicalSize = const Size(1600, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (_) =>
            MaterialPageRoute<void>(builder: (_) => SeasonPage(api: api)),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Map<String, dynamic> _episode(String id, int number) => <String, dynamic>{
  'id': id,
  'season_id': 'se1',
  'number': number,
  'title': 'Episode $number',
  'series_id': 's1',
  'series_title': 'The Show',
  'season_number': 1,
};

List<Map<String, dynamic>> _states(
  String body,
  String? inProgressId,
  int percent,
) {
  final List<dynamic> titles =
      (jsonDecode(body) as Map<String, dynamic>)['titles'] as List<dynamic>;
  return <Map<String, dynamic>>[
    for (final dynamic t in titles)
      <String, dynamic>{
        'title': t,
        'watched': false,
        'progress_percent': (t as Map<String, dynamic>)['id'] == inProgressId
            ? percent
            : 0,
      },
  ];
}

ApiClient _api(
  List<String> seen, {
  List<Map<String, dynamic>> episodes = const <Map<String, dynamic>>[],
  String? inProgress,
  int percent = 40,
}) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    seen.add(req.url.path);
    if (req.url.path == '/api/v1/users/self') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': 'user',
        }),
        200,
      );
    }
    if (req.url.path.endsWith('/state/batch')) {
      return http.Response(
        jsonEncode(_states(req.body, inProgress, percent)),
        200,
      );
    }
    if (req.url.path.endsWith('/episodes')) {
      return http.Response(jsonEncode(episodes), 200);
    }
    if (req.url.path.contains('/seasons/')) {
      return http.Response(jsonEncode(_season()), 200);
    }
    return http.Response(jsonEncode(<dynamic>[]), 200);
  }),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('an in-progress episode carries its progress into the list', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(
      tester,
      _api(
        seen,
        episodes: <Map<String, dynamic>>[_episode('e1', 1), _episode('e2', 2)],
        inProgress: 'e2',
      ),
    );

    final Iterable<CatalogCardTile> tiles = tester.widgetList<CatalogCardTile>(
      find.byType(CatalogCardTile),
    );
    expect(
      tiles
          .where((CatalogCardTile t) => t.card.progressPercent != null)
          .map((CatalogCardTile t) => t.card.ref.id),
      <String>['e2'],
      reason: 'only the episode the state batch reported is marked',
    );
    expect(
      seen.where((String p) => p.endsWith('/continue')),
      isEmpty,
      reason:
          'a season holds at most a few dozen episodes, so it asks about '
          'those rather than about every title in the catalog',
    );
  });

  testWidgets('the series crumb comes from the season, with no second fetch', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(tester, _api(seen));

    expect(find.text('The Show'), findsOneWidget);
    expect(find.text('Specials'), findsWidgets);
    expect(
      seen.where((String p) => RegExp(r'^/api/v1/series/[^/]+$').hasMatch(p)),
      isEmpty,
    );
  });

  testWidgets('marking the season watched reloads the episode cards', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(tester, _api(seen));

    int episodeLoads() =>
        seen.where((String p) => p.endsWith('/episodes')).length;
    expect(episodeLoads(), 1);

    final Finder watched = find.widgetWithText(
      ToggleButton,
      Strings.watchedLabel,
    );
    await tester.ensureVisible(watched);
    await tester.pumpAndSettle();
    await tester.tap(watched);
    await tester.pumpAndSettle();

    expect(
      seen.where((String p) => p.startsWith('/api/v1/users/u1/watched/')),
      isNotEmpty,
    );
    expect(episodeLoads(), 2);
    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a season with no episodes offers nothing to randomise', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]));
    expect(find.byType(RandomButton), findsNothing);
  });

  testWidgets(
    'random play sits on the Episodes heading, not with the toggles',
    (WidgetTester tester) async {
      final List<String> seen = <String>[];
      await _pump(
        tester,
        _api(
          seen,
          episodes: <Map<String, dynamic>>[
            _episode('e1', 1),
            _episode('e2', 2),
          ],
        ),
      );

      final Finder random = find.byType(RandomButton);
      expect(random, findsOneWidget);

      final Finder heading = find.textContaining(Strings.episodesHeading);
      final Rect headingBox = tester.getRect(heading);
      final Rect randomBox = tester.getRect(random);

      expect(
        randomBox.center.dy,
        moreOrLessEquals(headingBox.center.dy, epsilon: 14),
        reason: 'the button shares the heading row',
      );
      expect(randomBox.left, greaterThan(headingBox.left));

      final Rect toggles = tester.getRect(
        find.widgetWithText(ToggleButton, Strings.watchedLabel),
      );
      expect(
        randomBox.top,
        greaterThan(toggles.bottom),
        reason: 'it moved off the toggles row and down to the episodes heading',
      );
    },
  );
}
