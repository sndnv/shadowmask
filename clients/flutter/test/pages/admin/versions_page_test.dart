import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/versions_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

http.Response _route(http.Request req) {
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
    return http.Response(
      jsonEncode(<String, dynamic>{
        'items': <dynamic>[
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
        ],
        'total': 2,
        'offset': 0,
        'limit': 50,
      }),
      200,
    );
  }
  if (path == '/api/v1/versions/v1') {
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
            'language': 'eng',
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

Future<void> _pump(WidgetTester tester, {List<String>? seen}) async {
  tester.view.physicalSize = const Size(1600, 1000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request r) async {
      seen?.add('${r.method} ${r.url.path}');
      return _route(r);
    }),
  );
  await tester.pumpWidget(_app(api));
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the table lists versions and a row opens its detail page', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text('/media/bbb.mkv'), findsOneWidget);
    expect(find.text('1200 MB'), findsOneWidget);
    expect(find.text(Strings.relink), findsNothing);

    await tester.tap(find.text('/media/bbb.mkv'));
    await tester.pumpAndSettle();

    expect(find.text('went to /version?id=v1'), findsOneWidget);
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

    expect(seen, contains('DELETE /api/v1/admin/versions/v1'));
    expect(find.text(Strings.toastVersionRemoved), findsOneWidget);
    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('the filter narrows the table and reports an empty match', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await tester.enterText(find.byType(TextField), 'sintel');
    await tester.pumpAndSettle();

    expect(find.text('/media/sintel.mp4'), findsOneWidget);
    expect(find.text('/media/bbb.mkv'), findsNothing);

    await tester.enterText(find.byType(TextField), 'Films');
    await tester.pumpAndSettle();

    expect(find.text('/media/bbb.mkv'), findsOneWidget);
    expect(find.text('/media/sintel.mp4'), findsNothing);

    await tester.enterText(find.byType(TextField), 'nothing here');
    await tester.pumpAndSettle();

    expect(find.text(Strings.noMatchingVersions), findsOneWidget);
  });
}
