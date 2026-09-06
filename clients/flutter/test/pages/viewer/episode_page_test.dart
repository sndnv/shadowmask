import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/poster_play.dart';
import 'package:shadowmask/components/version_menu.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/viewer/episode_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _episode() => <String, dynamic>{
  'id': 'e1',
  'season_id': 'se1',
  'series_id': 's1',
  'series_title': 'The Show',
  'season_number': 0,
  'season_title': 'Specials',
  'number': 3,
  'title': 'A Christmas Special',
  'added_at': '2026-08-17T09:00:00Z',
  'updated_at': '2026-08-17T09:00:00Z',
  'artwork': <String, dynamic>{},
};

Map<String, dynamic> _version(String id, String label) => <String, dynamic>{
  'id': id,
  'title': <String, dynamic>{'type': 'episode', 'id': 'e1'},
  'library_id': 'lib1',
  'path': '/media/$id.mkv',
  'container': 'mkv',
  'quality': label,
  'available': true,
  'added_at': '2026-08-17T09:00:00Z',
  'updated_at': '2026-08-17T09:00:00Z',
};

Map<String, dynamic> _inProgress(String versionId) => <String, dynamic>{
  'card': <String, dynamic>{
    'title': <String, dynamic>{'type': 'episode', 'id': 'e1'},
    'display_title': 'A Christmas Special',
    'artwork': <String, dynamic>{},
    'duration_ms': 2400000,
    'progress_percent': 35,
  },
  'progress': <String, dynamic>{'version_id': versionId},
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
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) => settings.name == null || settings.name == '/'
              ? EpisodePage(api: api, id: 'e1')
              : Text('went to ${settings.name}'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

ApiClient _api(
  List<String> seen, {
  List<Map<String, dynamic>> versions = const <Map<String, dynamic>>[],
  List<Map<String, dynamic>> resuming = const <Map<String, dynamic>>[],
  bool admin = false,
}) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    seen.add(req.url.path);
    if (req.url.path == '/api/v1/users/self') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': admin ? 'admin' : 'user',
        }),
        200,
      );
    }
    if (req.url.path.endsWith('/versions')) {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'items': versions,
          'total': versions.length,
          'offset': 0,
          'limit': 50,
        }),
        200,
      );
    }
    if (req.url.path.endsWith('/continue')) {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'now_playing': <dynamic>[],
          'in_progress': resuming,
          'next_episodes': <dynamic>[],
        }),
        200,
      );
    }
    if (req.url.path.contains('/episodes/')) {
      return http.Response(jsonEncode(_episode()), 200);
    }
    return http.Response(jsonEncode(<dynamic>[]), 200);
  }),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('both crumbs come from the episode, with no parent fetches', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(tester, _api(seen));

    expect(find.text('The Show'), findsOneWidget);
    expect(find.text('Specials'), findsOneWidget);
    expect(
      find.byTooltip(Strings.relink),
      findsNothing,
      reason: 'a show is relinked as a whole, from the series page',
    );
    expect(
      seen.where((String p) => RegExp(r'^/api/v1/series/[^/]+$').hasMatch(p)),
      isEmpty,
    );
    expect(
      seen.where(
        (String p) =>
            RegExp(r'^/api/v1/series/[^/]+/seasons/[^/]+$').hasMatch(p),
      ),
      isEmpty,
    );
  });

  testWidgets('a single version plays straight from the artwork', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(<String>[], versions: <Map<String, dynamic>>[_version('v1', 'fhd')]),
    );

    await tester.tap(find.byType(PosterPlay));
    await tester.pumpAndSettle();

    expect(find.text('went to /watch?version=v1'), findsOneWidget);
  });

  testWidgets('the artwork opens the version menu when nothing resumes', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        <String>[],
        versions: <Map<String, dynamic>>[
          _version('v1', 'fhd'),
          _version('v2', 'sd'),
        ],
      ),
    );

    expect(find.byType(VersionMenuItem), findsNothing);

    await tester.tap(find.byType(PosterPlay));
    await tester.pumpAndSettle();

    expect(find.byType(VersionMenuItem), findsNWidgets(2));
  });

  testWidgets('the artwork resumes the version that is in progress', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        <String>[],
        versions: <Map<String, dynamic>>[
          _version('v1', 'fhd'),
          _version('v2', 'sd'),
        ],
        resuming: <Map<String, dynamic>>[_inProgress('v2')],
      ),
    );

    await tester.tap(find.byType(PosterPlay));
    await tester.pumpAndSettle();

    expect(find.text('went to /watch?version=v2'), findsOneWidget);
  });

  testWidgets('the artwork is not interactive without an available version', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]));

    expect(find.byType(PosterPlay), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(PosterPlay),
        matching: find.byType(InkWell),
      ),
      findsNothing,
    );
  });

  testWidgets('an episode with nothing to play says so', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]));

    expect(find.text(Strings.noVersionsAvailable), findsOneWidget);
  });

  testWidgets('an episode that plays says nothing of the sort', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(<String>[], versions: <Map<String, dynamic>>[_version('v1', 'fhd')]),
    );

    expect(find.text(Strings.noVersionsAvailable), findsNothing);
  });
}
