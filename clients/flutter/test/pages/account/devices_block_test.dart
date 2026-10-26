import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/devices_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

class _Server {
  final List<Map<String, dynamic>> shares = <Map<String, dynamic>>[
    <String, dynamic>{'user_id': 'u2', 'username': 'kid'},
  ];
  final List<http.Request> writes = <http.Request>[];

  http.Response answer(http.Request req) {
    final String path = req.url.path;
    if (req.method == 'GET' && path == '/api/v1/users/u1/devices') {
      return http.Response(
        jsonEncode(<dynamic>[
          <String, dynamic>{
            'id': 'd1',
            'user_id': 'u1',
            'name': 'Lounge tablet',
            'platform': 'android',
            'created_at': '2026-08-17T09:00:00Z',
            'shared_with': shares,
          },
        ]),
        200,
      );
    }
    writes.add(req);
    if (req.method == 'PUT' && path == '/api/v1/devices/d1/shares') {
      final Map<String, dynamic> body =
          jsonDecode(req.body) as Map<String, dynamic>;
      if (body['username'] != 'sam') {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'error': <String, dynamic>{
              'code': 'user_not_found',
              'message': 'no user has that username',
            },
          }),
          404,
        );
      }
      final Map<String, dynamic> shared = <String, dynamic>{
        'user_id': 'u3',
        'username': 'sam',
      };
      shares.add(shared);
      return http.Response(jsonEncode(shared), 200);
    }
    if (req.method == 'DELETE' && path == '/api/v1/devices/d1/shares/u2') {
      shares.removeWhere((Map<String, dynamic> s) => s['user_id'] == 'u2');
      return http.Response('', 204);
    }
    return http.Response('', 404);
  }
}

Future<_Server> _pump(WidgetTester tester) async {
  final _Server server = _Server();
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async => server.answer(req)),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child!),
        home: Scaffold(
          body: SingleChildScrollView(
            child: DevicesBlock(api: api, userId: 'u1'),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return server;
}

Future<void> _openShare(WidgetTester tester) async {
  await tester.tap(find.widgetWithText(TextButton, Strings.share));
  await tester.pumpAndSettle();
}

Future<void> _submitShare(WidgetTester tester, String username) async {
  await tester.enterText(fieldNamed(Strings.fieldUsername), username);
  await tester.tap(find.widgetWithText(FilledButton, Strings.share));
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a device lists the users it is shared with', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text(Strings.sharedWith('kid')), findsOneWidget);
  });

  testWidgets('sharing by username adds the user under the device', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester);
    await _openShare(tester);

    expect(find.text(Strings.shareDeviceTitle('Lounge tablet')), findsOne);

    await _submitShare(tester, ' sam ');

    expect(server.writes.single.method, 'PUT');
    expect(jsonDecode(server.writes.single.body), <String, dynamic>{
      'username': 'sam',
    });
    expect(find.text(Strings.shareDeviceTitle('Lounge tablet')), findsNothing);
    expect(find.text(Strings.toastDeviceShared('sam')), findsOneWidget);
    expect(find.text(Strings.sharedWith('sam')), findsOneWidget);
  });

  testWidgets('an unknown username keeps the dialog open and says so', (
    WidgetTester tester,
  ) async {
    await _pump(tester);
    await _openShare(tester);

    await _submitShare(tester, 'nobody');

    expect(
      find.text('${Strings.errorShare} ${Strings.reasonUserNotFound}'),
      findsOneWidget,
    );
    expect(find.text(Strings.shareDeviceTitle('Lounge tablet')), findsOne);
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, Strings.share),
          )
          .onPressed,
      isNotNull,
      reason: 'the user can correct the name and try again',
    );
  });

  testWidgets('an empty username is refused before any request', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester);
    await _openShare(tester);

    await _submitShare(tester, '  ');

    expect(find.text(Strings.requiredUsername), findsOneWidget);
    expect(server.writes, isEmpty);
  });

  testWidgets('removing a share takes the user off the device', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester);

    await tester.tap(find.widgetWithText(TextButton, Strings.remove));
    await tester.pumpAndSettle();

    expect(server.writes.single.method, 'DELETE');
    expect(find.text(Strings.toastShareRemoved), findsOneWidget);
    expect(find.text(Strings.sharedWith('kid')), findsNothing);
  });
}
