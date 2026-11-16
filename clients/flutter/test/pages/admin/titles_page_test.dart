import 'dart:async';
import 'dart:convert';

import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/admin_filter_field.dart';
import 'package:shadowmask/components/menu_option.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/titles_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

const String _added = '2026-11-01T10:00:00Z';

const List<Map<String, dynamic>> _rows = <Map<String, dynamic>>[
  <String, dynamic>{
    'kind': 'movie',
    'id': 'm1',
    'title': 'Paper Skies',
    'year': 2018,
    'files': 2,
    'available': true,
    'readable': true,
    'added_at': _added,
  },
  <String, dynamic>{
    'kind': 'movie',
    'id': 'm2',
    'title': 'Lantern',
    'year': 2020,
    'files': 0,
    'available': false,
    'readable': false,
    'added_at': _added,
  },
  <String, dynamic>{
    'kind': 'movie',
    'id': 'm3',
    'title': 'Dusk',
    'year': 2021,
    'files': 3,
    'available': true,
    'readable': false,
    'added_at': _added,
  },
  <String, dynamic>{
    'kind': 'series',
    'id': 's1',
    'title': 'Skyline',
    'year': 2019,
    'files': 1,
    'episodes': 3,
    'seasons': 2,
    'available': false,
    'readable': true,
    'added_at': _added,
  },
  <String, dynamic>{
    'kind': 'season',
    'id': 'se1',
    'series_id': 's1',
    'series_title': 'Skyline',
    'season_number': 1,
    'files': 1,
    'episodes': 2,
    'available': false,
    'readable': true,
    'added_at': _added,
  },
  <String, dynamic>{
    'kind': 'episode',
    'id': 'e1',
    'series_id': 's1',
    'season_id': 'se1',
    'title': 'Earth',
    'series_title': 'Skyline',
    'season_number': 1,
    'episode_number': 2,
    'files': 1,
    'available': false,
    'readable': true,
    'added_at': _added,
  },
  <String, dynamic>{
    'kind': 'episode',
    'id': 'e2',
    'series_id': 's1',
    'season_id': 'se1',
    'title': 'Wind',
    'series_title': 'Skyline',
    'season_number': 1,
    'episode_number': 3,
    'files': 0,
    'available': false,
    'readable': false,
    'added_at': _added,
  },
  <String, dynamic>{
    'kind': 'season',
    'id': 'se2',
    'series_id': 's1',
    'title': 'Finale',
    'series_title': 'Skyline',
    'season_number': 2,
    'files': 0,
    'episodes': 0,
    'available': false,
    'readable': false,
    'added_at': _added,
  },
  <String, dynamic>{
    'kind': 'series',
    'id': 's2',
    'title': 'Bare',
    'files': 0,
    'episodes': 0,
    'seasons': 0,
    'available': false,
    'readable': false,
    'added_at': _added,
  },
];

String _availabilityOf(Map<String, dynamic> row) => row['available'] == true
    ? 'available'
    : row['files'] == 0
    ? 'no_version'
    : 'unavailable';

bool _named(Map<String, dynamic> row, String needle) =>
    '${row['title'] ?? ''} ${row['series_title'] ?? ''}'.toLowerCase().contains(
      needle.toLowerCase(),
    );

http.Response _page(
  http.Request req,
  int pageSize,
  List<Map<String, dynamic>> rows,
) {
  final Map<String, String> q = req.url.queryParameters;
  final int offset = int.tryParse(q['offset'] ?? '') ?? 0;
  final List<Map<String, dynamic>> matched = rows
      .where((Map<String, dynamic> r) => _named(r, q['filter'] ?? ''))
      .where(
        (Map<String, dynamic> r) => q['kind'] == null || r['kind'] == q['kind'],
      )
      .where(
        (Map<String, dynamic> r) =>
            q['availability'] == null ||
            _availabilityOf(r) == q['availability'],
      )
      .toList();
  return http.Response(
    jsonEncode(<String, dynamic>{
      'items': matched.skip(offset).take(pageSize).toList(),
      'total': matched.length,
      'offset': offset,
      'limit': pageSize,
    }),
    200,
  );
}

Widget _app(ApiClient api) => ThemeScope(
  variant: AppThemeVariant.dark,
  setVariant: (_) {},
  child: MaterialApp(
    theme: buildTheme(AppThemeVariant.dark),
    builder: (BuildContext context, Widget? child) =>
        ToastHost(child: child ?? const SizedBox.shrink()),
    onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
      builder: (_) => settings.name == null || settings.name == '/'
          ? TitlesPage(api: api)
          : Text('went to ${settings.name}'),
    ),
  ),
);

Future<void> _pump(
  WidgetTester tester, {
  List<String>? seen,
  List<Uri>? reads,
  int pageSize = 50,
  List<Map<String, dynamic>> rows = _rows,
  bool forget = false,
  bool versionsFail = false,
  Completer<void>? holdVersions,
  String role = 'admin',
  Size size = const Size(1600, 1000),
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final List<Map<String, dynamic>> live = List<Map<String, dynamic>>.of(rows);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request r) async {
      seen?.add('${r.method} ${r.url.path}');
      final String path = r.url.path;
      if (path == '/api/v1/users/self') {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'id': 'u1',
            'username': 'root',
            'role': role,
          }),
          200,
        );
      }
      if (path == '/api/v1/admin/titles') {
        reads?.add(r.url);
        return _page(r, pageSize, live);
      }
      if (forget && r.method == 'DELETE') {
        live.removeWhere(
          (Map<String, dynamic> row) => path.endsWith('/${row['id']}'),
        );
      }
      if (path.endsWith('/versions')) {
        await holdVersions?.future;
        if (versionsFail) {
          return http.Response('', 500);
        }
        return http.Response(
          jsonEncode(<String, dynamic>{
            'items': <dynamic>[
              <String, dynamic>{
                'id': 'v1',
                'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
                'library_id': 'lib',
                'quality': 'hd',
                'container': 'mkv',
                'path': '/media/paper-skies.mkv',
              },
            ],
            'total': 1,
            'offset': 0,
            'limit': 50,
          }),
          200,
        );
      }
      return http.Response('', 204);
    }),
  );
  await tester.pumpWidget(_app(api));
  await tester.pumpAndSettle();
}

Future<void> _type(WidgetTester tester, String text) async {
  await tester.enterText(find.byType(AdminFilterField), text);
  await tester.pump(kFilterDebounce + const Duration(milliseconds: 50));
  await tester.pumpAndSettle();
}

Future<void> _choose(WidgetTester tester, String current, String wanted) async {
  await tester.tap(find.text(current).first);
  await tester.pumpAndSettle();
  await tester.tap(
    find.descendant(of: find.byType(MenuOption), matching: find.text(wanted)),
  );
  await tester.pumpAndSettle();
}

Future<void> _next(WidgetTester tester, List<Uri> reads) async {
  await tester.tap(find.text(Strings.next));
  await tester.pumpAndSettle();
  expect(reads.last.queryParameters.containsKey('offset'), isTrue);
}

Future<void> _delete(WidgetTester tester, String tooltip, String named) async {
  await tester.tap(find.byTooltip(tooltip));
  await tester.pumpAndSettle();
  expect(find.text(Strings.confirmDeleteTitle(named)), findsOneWidget);
  await tester.tap(find.widgetWithText(FilledButton, Strings.delete));
  await tester.pumpAndSettle();
  await tester.pump(kToastDuration + const Duration(milliseconds: 100));
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets(
    'every kind is named its own way, with its context and versions',
    (WidgetTester tester) async {
      await _pump(tester);

      for (final String name in <String>[
        'Paper Skies',
        'Lantern',
        'Dusk',
        'Season 1',
        'Finale',
        'S01E02 Earth',
        'S01E03 Wind',
        'Bare',
      ]) {
        expect(find.text(name), findsOneWidget, reason: name);
      }
      expect(
        find.text('Skyline'),
        findsNWidgets(3),
        reason: 'the series itself, then under each of its two seasons',
      );
      expect(find.text('Skyline · Season 1'), findsNWidgets(2));
      expect(find.text('3 versions'), findsOneWidget);
      expect(find.text('2 versions'), findsOneWidget);
      expect(find.text('1 version'), findsOneWidget);
      expect(find.text('0 versions'), findsNWidgets(2));
      expect(find.text('1 of 3 episodes'), findsOneWidget);
      expect(find.text('1 of 2 episodes'), findsOneWidget);
      expect(find.text(Strings.noEpisodes), findsNWidgets(2));
      expect(find.text(Strings.availabilityAvailable), findsNothing);
      expect(find.text(Strings.availabilityNoVersion), findsNothing);
      expect(find.text(Strings.kindSeason), findsNWidgets(2));
      expect(find.text('2018'), findsOneWidget);
      expect(find.text('2026-11-01'), findsNWidgets(_rows.length));
      expect(
        find.text(Strings.countLabel(Strings.adminTitles, 9)),
        findsOneWidget,
      );
    },
  );

  testWidgets('the row tint follows availability', (WidgetTester tester) async {
    await _pump(tester);

    expect(rowTintOf(tester, 'Paper Skies'), Tokens.dark.rowOk);
    expect(rowTintOf(tester, 'S01E02 Earth'), Tokens.dark.rowDanger);
    expect(rowTintOf(tester, 'Lantern'), Tokens.dark.rowWarn);
  });

  testWidgets('a readable episode opens its page, and a row the server marks '
      'unreadable does not', (WidgetTester tester) async {
    await _pump(tester);

    await tester.tap(find.text('Lantern'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Dusk'));
    await tester.pumpAndSettle();
    expect(
      find.textContaining('went to'),
      findsNothing,
      reason: 'Dusk has files but is past the cap, so only readable opens',
    );

    await tester.tap(find.text('S01E02 Earth'));
    await tester.pumpAndSettle();
    expect(
      find.text('went to /episode?id=e1&season=se1&series=s1'),
      findsOneWidget,
    );
  });

  testWidgets('a readable season opens its page under its series', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await tester.tap(find.text('Season 1'));
    await tester.pumpAndSettle();

    expect(find.text('went to /season?id=se1&series=s1'), findsOneWidget);
  });

  testWidgets('delete is offered on an empty row and names what blocks the '
      'others', (WidgetTester tester) async {
    await _pump(tester);

    for (final String blocked in <String>[
      Strings.blockedByVersions(2),
      Strings.blockedBySeasons(2),
      Strings.blockedByEpisodes(2),
      Strings.blockedByVersions(1),
    ]) {
      final IconButton button = tester.widget<IconButton>(
        find.ancestor(
          of: find.byTooltip(blocked),
          matching: find.byType(IconButton),
        ),
      );
      expect(button.onPressed, isNull, reason: blocked);
    }
    for (final String open in <String>[
      Strings.deleteMovie,
      Strings.deleteSeason,
      Strings.deleteEpisode,
      Strings.deleteSeries,
    ]) {
      expect(find.byTooltip(open), findsOneWidget, reason: open);
    }
  });

  testWidgets('clicking a blocked delete opens nothing', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await tester.tap(find.byTooltip(Strings.blockedByVersions(2)));
    await tester.pumpAndSettle();

    expect(find.textContaining('went to'), findsNothing);
  });

  testWidgets('a confirmed delete calls its kind\'s endpoint and reloads', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    final List<Uri> reads = <Uri>[];
    await _pump(tester, seen: seen, reads: reads);

    await tester.tap(find.byTooltip(Strings.deleteMovie));
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmDeleteTitle('Lantern')), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, Strings.delete));
    await tester.pumpAndSettle();
    expect(find.text(Strings.toastDeleted), findsOneWidget);
    await tester.pump(kToastDuration + const Duration(milliseconds: 100));

    await _delete(tester, Strings.deleteSeason, 'Skyline · Finale');
    await _delete(tester, Strings.deleteEpisode, 'Skyline · S01E03 Wind');
    await _delete(tester, Strings.deleteSeries, 'Bare');

    expect(seen.where((String call) => call.startsWith('DELETE')), <String>[
      'DELETE /api/v1/admin/movies/m2',
      'DELETE /api/v1/admin/seasons/se2',
      'DELETE /api/v1/admin/episodes/e2',
      'DELETE /api/v1/admin/series/s2',
    ]);
    expect(reads.length, 5, reason: 'the first read and one after each delete');
  });

  testWidgets('emptying the last page steps back to the one before', (
    WidgetTester tester,
  ) async {
    final List<Uri> reads = <Uri>[];
    final List<Map<String, dynamic>> rows = <Map<String, dynamic>>[
      for (final String title in <String>['Alder', 'Birch'])
        <String, dynamic>{
          'kind': 'movie',
          'id': title.toLowerCase(),
          'title': title,
          'files': 0,
          'added_at': _added,
        },
    ];
    await _pump(tester, reads: reads, rows: rows, forget: true, pageSize: 1);
    await _next(tester, reads);
    expect(find.text('Birch'), findsOneWidget);

    await _delete(tester, Strings.deleteMovie, 'Birch');

    expect(reads.last.queryParameters.containsKey('offset'), isFalse);
    expect(find.text('Alder'), findsOneWidget);
    expect(find.text(Strings.emptyTitles), findsNothing);
  });

  testWidgets('a cancelled delete sends nothing', (WidgetTester tester) async {
    final List<String> seen = <String>[];
    await _pump(tester, seen: seen);

    await tester.tap(find.byTooltip(Strings.deleteMovie));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.cancel));
    await tester.pumpAndSettle();

    expect(seen.where((String call) => call.startsWith('DELETE')), isEmpty);
  });

  testWidgets('the filter is sent once typing settles, from the first page', (
    WidgetTester tester,
  ) async {
    final List<Uri> reads = <Uri>[];
    await _pump(tester, reads: reads, pageSize: 2);
    await tester.tap(find.text(Strings.next));
    await tester.pumpAndSettle();
    expect(reads.last.queryParameters['offset'], '2');

    final int before = reads.length;
    await tester.enterText(find.byType(AdminFilterField), 'w');
    await tester.pump(const Duration(milliseconds: 50));
    await _type(tester, 'wind');

    expect(reads.length, before + 1, reason: 'one request once typing settles');
    expect(reads[before].queryParameters['filter'], 'wind');
    expect(reads[before].queryParameters.containsKey('offset'), isFalse);
    expect(find.text('S01E03 Wind'), findsOneWidget);
    expect(find.text('Paper Skies'), findsNothing);
  });

  testWidgets('the kind and availability are sent, each from the first page, '
      'with the filter kept', (WidgetTester tester) async {
    final List<Uri> reads = <Uri>[];
    await _pump(tester, reads: reads, pageSize: 1);
    await _type(tester, 'sky');
    await _next(tester, reads);

    int changed = reads.length;
    await _choose(tester, Strings.availabilityAll, Strings.typeSeason);
    expect(reads[changed].queryParameters['kind'], 'season');
    expect(reads[changed].queryParameters['filter'], 'sky');
    expect(reads[changed].queryParameters.containsKey('offset'), isFalse);
    expect(find.text('Season 1'), findsOneWidget);
    expect(find.text('S01E02 Earth'), findsNothing);

    await _next(tester, reads);
    changed = reads.length;
    await _choose(
      tester,
      Strings.availabilityAll,
      Strings.availabilityNoVersion,
    );
    expect(reads[changed].queryParameters['availability'], 'no_version');
    expect(reads[changed].queryParameters['kind'], 'season');
    expect(reads[changed].queryParameters.containsKey('offset'), isFalse);
    expect(find.text('Finale'), findsOneWidget);
    expect(find.text('Season 1'), findsNothing);

    await _choose(tester, Strings.typeSeason, Strings.availabilityAll);
    expect(reads.last.queryParameters.containsKey('kind'), isFalse);
  });

  testWidgets('an empty catalog and a filter with no match say so', (
    WidgetTester tester,
  ) async {
    await _pump(tester, rows: const <Map<String, dynamic>>[]);
    expect(find.text(Strings.emptyTitles), findsOneWidget);

    await _type(tester, 'nothing');
    expect(find.text(Strings.noMatchingTitles), findsOneWidget);
  });

  testWidgets('a phone keeps the context or the kind, and the count, under the '
      'name', (WidgetTester tester) async {
    await _pump(tester, size: const Size(400, 900));

    for (final String line in <String>[
      '${Strings.kindMovie} · 2 versions',
      '${Strings.kindSeries} · 1 of 3 episodes',
      'Skyline · 1 of 2 episodes',
      'Skyline · Season 1 · 1 version',
    ]) {
      expect(find.text(line), findsOneWidget, reason: line);
    }
    expect(find.text('Season 1'), findsOneWidget);
    expect(find.text('2018'), findsNothing);
  });

  testWidgets('a column header sorts on the server, from the first page', (
    WidgetTester tester,
  ) async {
    final List<Uri> reads = <Uri>[];
    await _pump(tester, reads: reads, pageSize: 2);
    expect(reads.last.queryParameters.containsKey('sort'), isFalse);

    for (final (String header, String sort) in <(String, String)>[
      (Strings.columnKind, 'kind'),
      (Strings.columnYear, 'year'),
      (Strings.versionsHeading, 'versions'),
      (Strings.columnAdded, 'added'),
    ]) {
      await _next(tester, reads);
      final int changed = reads.length;
      await tester.tap(find.text(header.toUpperCase()));
      await tester.pumpAndSettle();
      expect(reads[changed].queryParameters['sort'], sort, reason: header);
      expect(reads[changed].queryParameters.containsKey('order'), isFalse);
      expect(reads[changed].queryParameters.containsKey('offset'), isFalse);
    }

    await tester.tap(find.text(Strings.columnAdded.toUpperCase()));
    await tester.pumpAndSettle();
    expect(reads.last.queryParameters['sort'], 'added');
    expect(reads.last.queryParameters['order'], 'desc');

    await tester.tap(find.text(Strings.columnAdded.toUpperCase()));
    await tester.pumpAndSettle();
    expect(reads.last.queryParameters['sort'], 'added');
    expect(
      reads.last.queryParameters.containsKey('order'),
      isFalse,
      reason:
          'the reloaded table remembers it was descending, so this turns it',
    );

    await tester.tap(find.text(Strings.columnTitle.toUpperCase()));
    await tester.pumpAndSettle();
    expect(reads.last.queryParameters.containsKey('sort'), isFalse);
    expect(reads.last.queryParameters.containsKey('order'), isFalse);
  });

  testWidgets('a readable movie or episode lists its versions in a dialog', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(tester, seen: seen);

    expect(
      find.byTooltip(Strings.versionsHeading),
      findsNWidgets(2),
      reason: 'Paper Skies and Earth; the rest have no page or no versions',
    );

    await tester.tap(find.byTooltip(Strings.versionsHeading).first);
    await tester.pumpAndSettle();
    expect(seen, contains('GET /api/v1/movies/m1/versions'));
    expect(
      find.text(Strings.countLabel(Strings.versionsHeading, 1)),
      findsOneWidget,
    );
    await tester.tap(find.byTooltip(Strings.close).last);
    await tester.pumpAndSettle();

    await tester.tap(find.byTooltip(Strings.versionsHeading).last);
    await tester.pumpAndSettle();
    expect(
      seen,
      contains('GET /api/v1/series/s1/seasons/se1/episodes/e1/versions'),
    );
  });

  testWidgets('a second click while the versions load opens nothing else', (
    WidgetTester tester,
  ) async {
    final Completer<void> hold = Completer<void>();
    await _pump(tester, holdVersions: hold);

    await tester.tap(find.byTooltip(Strings.versionsHeading).first);
    await tester.pump();
    await tester.tap(find.byTooltip(Strings.versionsHeading).first);
    await tester.pumpAndSettle();
    expect(find.textContaining('went to'), findsNothing);

    hold.complete();
    await tester.pumpAndSettle();
    expect(
      find.text(Strings.countLabel(Strings.versionsHeading, 1)),
      findsOneWidget,
    );
  });

  testWidgets('versions that cannot be read say so', (
    WidgetTester tester,
  ) async {
    await _pump(tester, versionsFail: true);

    await tester.tap(find.byTooltip(Strings.versionsHeading).first);
    await tester.pumpAndSettle();

    expect(find.textContaining(Strings.couldNotLoadVersions), findsOneWidget);
    expect(
      find.text(Strings.countLabel(Strings.versionsHeading, 1)),
      findsNothing,
    );
    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('only an admin sees the list', (WidgetTester tester) async {
    final List<Uri> reads = <Uri>[];
    await _pump(tester, reads: reads, role: 'user');

    expect(find.text(Strings.notAuthorized), findsOneWidget);
    expect(reads, isEmpty);
  });
}
