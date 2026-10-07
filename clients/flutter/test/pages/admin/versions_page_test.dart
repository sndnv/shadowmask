import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/admin_filter_field.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/versions_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

const List<Map<String, dynamic>> _versions = <Map<String, dynamic>>[
  <String, dynamic>{
    'id': 'v1',
    'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
    'library_id': 'lib',
    'quality': 'hd',
    'container': 'mkv',
    'size_bytes': 1258291200,
    'path': '/media/bbb.mkv',
  },
  <String, dynamic>{
    'id': 'v2',
    'title': <String, dynamic>{'type': 'movie', 'id': 'm2'},
    'library_id': 'gone',
    'quality': 'sd',
    'container': 'mp4',
    'size_bytes': 104857600,
    'available': false,
    'path': '/media/sintel.mp4',
  },
];

const Map<String, dynamic> _episodeCard = <String, dynamic>{
  'type': 'episode',
  'id': 'e1',
  'season_id': 'se2',
  'series_id': 's1',
  'number': 2,
  'title': 'Earth',
  'series_title': 'Skyline',
  'season_number': 2,
};

bool _hit(Map<String, dynamic> v, String needle) =>
    '${v['path']} ${v['library_id'] == 'lib' ? 'Films' : v['library_id']} '
            '${v['quality']} ${v['container']}'
        .toLowerCase()
        .contains(needle.toLowerCase());

http.Response _page(http.Request req, int pageSize) {
  final String needle = req.url.queryParameters['filter'] ?? '';
  final int offset = int.tryParse(req.url.queryParameters['offset'] ?? '') ?? 0;
  final List<Map<String, dynamic>> matched = _versions
      .where((Map<String, dynamic> v) => needle.isEmpty || _hit(v, needle))
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

http.Response _route(http.Request req, {int pageSize = 50}) {
  final String path = req.url.path;
  if (path == '/api/v1/users/self') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'id': 'u1',
        'username': 'root',
        'role': 'admin',
      }),
      200,
    );
  }
  if (path == '/api/v1/libraries') {
    return http.Response(
      jsonEncode(<dynamic>[
        <String, dynamic>{
          'id': 'lib',
          'name': 'Films',
          'kind': 'movie',
          'watcher': 'manual',
          'created_at': '2026-08-01T00:00:00Z',
          'updated_at': '2026-08-01T00:00:00Z',
        },
      ]),
      200,
    );
  }
  if (path == '/api/v1/admin/versions') {
    return _page(req, pageSize);
  }
  if (req.method == 'GET' && path == '/api/v1/versions/v1') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'id': 'v1',
        'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
        'library_id': 'lib',
        'quality': 'hd',
        'container': 'mkv',
        'duration_ms': 100000,
        'video': <dynamic>[
          <String, dynamic>{
            'index': 0,
            'codec': 'h264',
            'width': 1920,
            'height': 1080,
            'frame_rate': 24.0,
          },
        ],
        'audio': <dynamic>[
          <String, dynamic>{
            'index': 1,
            'codec': 'aac',
            'channels': 2,
            'language': 'en',
          },
        ],
        'subtitles': <dynamic>[],
      }),
      200,
    );
  }
  return http.Response('', 204);
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
          ? VersionsPage(api: api)
          : Text('went to ${settings.name}'),
    ),
  ),
);

Future<void> _pump(
  WidgetTester tester, {
  List<String>? seen,
  List<Uri>? reads,
  int pageSize = 50,
  Size size = const Size(1600, 1000),
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request r) async {
      seen?.add('${r.method} ${r.url.path}');
      if (r.url.path == '/api/v1/admin/versions') {
        reads?.add(r.url);
      }
      return _route(r, pageSize: pageSize);
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

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the table lists versions and a row opens its detail page', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text('/media/bbb.mkv'), findsOneWidget);
    expect(find.text('1.2 GB'), findsOneWidget);
    expect(find.text(Strings.relink), findsNothing);

    await tester.tap(find.text('/media/bbb.mkv'));
    await tester.pumpAndSettle();

    expect(find.text('went to /version?id=v1'), findsOneWidget);
  });

  testWidgets('an episode version is named by its series and code', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request r) async {
        if (r.url.path == '/api/v1/admin/versions') {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'items': <dynamic>[
                <String, dynamic>{
                  'id': 'v9',
                  'title': <String, dynamic>{'type': 'episode', 'id': 'e1'},
                  'library_id': 'lib',
                  'quality': 'hd',
                  'container': 'mkv',
                  'size_bytes': 1258291200,
                  'path': '/media/skyline.mkv',
                },
              ],
              'total': 1,
              'offset': 0,
              'limit': 50,
            }),
            200,
          );
        }
        if (r.url.path == '/api/v1/titles/batch') {
          return http.Response(jsonEncode(<dynamic>[_episodeCard]), 200);
        }
        return _route(r);
      }),
    );
    await tester.pumpWidget(_app(api));
    await tester.pumpAndSettle();

    expect(find.text('Skyline · S02E02'), findsOneWidget);
  });

  testWidgets('a phone shows the file name and keeps the path in the tooltip', (
    WidgetTester tester,
  ) async {
    await _pump(tester, size: const Size(400, 900));

    expect(find.text('bbb.mkv'), findsOneWidget);
    expect(find.text('/media/bbb.mkv'), findsNothing);
    expect(
      find.byWidgetPredicate(
        (Widget w) => w is Tooltip && w.message == '/media/bbb.mkv',
      ),
      findsOneWidget,
    );
  });

  testWidgets('the library column names the library, falling back to its id', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text('Films'), findsOneWidget);
    expect(find.text('gone'), findsOneWidget);
  });

  testWidgets('the row tint follows availability', (WidgetTester tester) async {
    await _pump(tester);

    expect(rowTintOf(tester, '/media/bbb.mkv'), Tokens.dark.rowOk);
    expect(rowTintOf(tester, '/media/sintel.mp4'), Tokens.dark.rowDanger);
  });

  testWidgets('every version offers removal, and it confirms first', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(tester, seen: seen);

    expect(find.byIcon(Icons.delete_outline), findsNWidgets(2));

    await tester.tap(find.byIcon(Icons.delete_outline).first);
    await tester.pumpAndSettle();
    expect(find.textContaining('/media/bbb.mkv'), findsWidgets);

    await tester.tap(find.widgetWithText(FilledButton, Strings.remove));
    await tester.pumpAndSettle();

    expect(seen, contains('DELETE /api/v1/versions/v1'));
    expect(find.text(Strings.toastVersionRemoved), findsOneWidget);
    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('the filter is sent to the server once typing settles', (
    WidgetTester tester,
  ) async {
    final List<Uri> reads = <Uri>[];
    await _pump(tester, reads: reads);
    expect(reads.single.queryParameters.containsKey('filter'), isFalse);

    await tester.enterText(find.byType(AdminFilterField), 'sint');
    await tester.pump(const Duration(milliseconds: 100));
    await tester.enterText(find.byType(AdminFilterField), 'sintel');
    await tester.pump(kFilterDebounce + const Duration(milliseconds: 50));
    await tester.pumpAndSettle();

    expect(reads, hasLength(2));
    expect(reads.last.queryParameters['filter'], 'sintel');
    expect(find.text('/media/sintel.mp4'), findsOneWidget);
    expect(find.text('/media/bbb.mkv'), findsNothing);
    expect(
      find.text(Strings.countLabel(Strings.adminVersions, 1)),
      findsOneWidget,
    );
  });

  testWidgets('a library name finds its versions', (WidgetTester tester) async {
    await _pump(tester);

    await _type(tester, 'Films');

    expect(find.text('/media/bbb.mkv'), findsOneWidget);
    expect(find.text('/media/sintel.mp4'), findsNothing);
  });

  testWidgets('a filter with no matches reports it', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await _type(tester, 'nothing here');

    expect(find.text(Strings.noMatchingVersions), findsOneWidget);
  });

  testWidgets('paging keeps the filter and asks for the next offset', (
    WidgetTester tester,
  ) async {
    final List<Uri> reads = <Uri>[];
    await _pump(tester, reads: reads, pageSize: 1);

    await _type(tester, 'media');
    expect(find.text('/media/bbb.mkv'), findsOneWidget);

    await tester.tap(find.text(Strings.next));
    await tester.pumpAndSettle();

    expect(reads.last.queryParameters['offset'], '1');
    expect(reads.last.queryParameters['filter'], 'media');
    expect(find.text('/media/sintel.mp4'), findsOneWidget);
    expect(find.text('/media/bbb.mkv'), findsNothing);
  });

  testWidgets('a new filter starts again from the first page', (
    WidgetTester tester,
  ) async {
    final List<Uri> reads = <Uri>[];
    await _pump(tester, reads: reads, pageSize: 1);

    await tester.tap(find.text(Strings.next));
    await tester.pumpAndSettle();
    expect(reads.last.queryParameters['offset'], '1');

    await _type(tester, 'media');

    expect(reads.last.queryParameters.containsKey('offset'), isFalse);
    expect(find.text('/media/bbb.mkv'), findsOneWidget);
  });

  testWidgets('the filter keeps focus while the reload it triggered runs', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await _type(tester, 'sintel');

    final EditableText field = tester.widget<EditableText>(
      find.descendant(
        of: find.byType(AdminFilterField),
        matching: find.byType(EditableText),
      ),
    );
    expect(field.focusNode.hasFocus, isTrue);
    expect(field.controller.text, 'sintel');
  });
}
