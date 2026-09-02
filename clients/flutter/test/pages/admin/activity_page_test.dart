import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/progress_meter.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/pages/admin/activity_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

Map<String, dynamic> _session(String id, String user, String state) =>
    <String, dynamic>{
      'session_id': id,
      'user_id': user,
      'version_id': 'v1',
      'mode': 'direct',
      'position_ms': 900000,
      'state': state,
      'started_at': '2026-08-17T07:24:11Z',
      'last_heartbeat_at': '2026-08-17T07:30:00Z',
    };

http.Response _route(http.Request req, [List<Map<String, dynamic>>? sessions]) {
  if (req.url.path == '/api/v1/users/activity' && sessions != null) {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'items': sessions,
        'total': sessions.length,
        'offset': 0,
        'limit': 20,
      }),
      200,
    );
  }
  if (req.url.path == '/api/v1/users/self') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'id': 'u1',
        'username': 'pat',
        'role': 'admin',
      }),
      200,
    );
  }
  if (req.url.path == '/api/v1/users') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'items': <dynamic>[
          <String, dynamic>{
            'id': 'u7',
            'username': 'sam',
            'role': 'user',
            'created_at': '2026-08-01T00:00:00Z',
            'updated_at': '2026-08-01T00:00:00Z',
          },
        ],
        'total': 1,
        'offset': 0,
        'limit': 200,
      }),
      200,
    );
  }
  if (req.url.path == '/api/v1/users/activity') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'items': <dynamic>[
          <String, dynamic>{
            'session_id': 's1',
            'user_id': 'u7',
            'version_id': 'v1',
            'mode': 'direct',
            'position_ms': 900000,
            'state': 'playing',
            'started_at': '2026-08-17T07:24:11Z',
            'last_heartbeat_at': '2026-08-17T07:30:00Z',
            'card': <String, dynamic>{
              'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
              'display_title': 'The Thing',
              'duration_ms': 6000000,
              'progress_percent': 42,
            },
          },
        ],
        'total': 1,
        'offset': 0,
        'limit': 20,
      }),
      200,
    );
  }
  return http.Response('{}', 200);
}

Future<void> _pump(
  WidgetTester tester, [
  List<Map<String, dynamic>>? sessions,
  Size size = const Size(1600, 1000),
]) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async => _route(req, sessions)),
  );

  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) => settings.name == null || settings.name == '/'
              ? ActivityPage(api: api)
              : Text('went to ${settings.name}'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a row names the user, links the title and shows progress', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text('sam'), findsOneWidget);
    expect(find.text('u7'), findsNothing);
    expect(find.byType(ProgressMeter), findsOneWidget);
    expect(find.text('42%'), findsOneWidget);

    await tester.tap(find.text('The Thing'));
    await tester.pumpAndSettle();

    expect(find.text('went to /title?type=movie&id=m1'), findsOneWidget);
  });

  testWidgets('the row tint separates playing from paused', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Map<String, dynamic>>[
      _session('s1', 'u7', 'playing'),
      _session('s2', 'u9', 'paused'),
    ]);

    expect(rowTintOf(tester, 'sam'), Tokens.dark.rowOk);
    expect(rowTintOf(tester, 'u9'), Tokens.dark.rowWarn);
  });

  testWidgets('a phone renders the table without overflowing', (
    WidgetTester tester,
  ) async {
    await _pump(tester, null, const Size(390, 1200));

    expect(tester.takeException(), isNull);
  });
}
