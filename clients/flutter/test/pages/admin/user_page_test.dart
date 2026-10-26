import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/user_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

Map<String, dynamic> _profile(String id, String username, String role) =>
    <String, dynamic>{
      'id': id,
      'username': username,
      'role': role,
      'preferred_audio': <String>[],
      'preferred_subtitle': <String>[],
      'created_at': '2026-01-01T00:00:00Z',
      'updated_at': '2026-01-01T00:00:00Z',
    };

class _Server {
  final List<http.Request> writes = <http.Request>[];
  int codeReads = 0;

  http.Response answer(http.Request req) {
    final String path = req.url.path;
    if (req.method != 'GET') {
      writes.add(req);
      return http.Response('', 204);
    }
    if (path == '/api/v1/users/self') {
      return http.Response(jsonEncode(_profile('u1', 'pat', 'admin')), 200);
    }
    if (path == '/api/v1/users/u1') {
      return http.Response(jsonEncode(_profile('u1', 'pat', 'admin')), 200);
    }
    if (path == '/api/v1/users/u2') {
      return http.Response(jsonEncode(_profile('u2', 'sam', 'user')), 200);
    }
    if (path == '/api/v1/users/u2/link-codes') {
      codeReads++;
    }
    return http.Response(jsonEncode(<dynamic>[]), 200);
  }
}

Future<_Server> _pump(WidgetTester tester, String id) async {
  tester.view.physicalSize = const Size(1600, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
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
        onGenerateRoute: (_) => MaterialPageRoute<void>(
          builder: (_) => UserPage(api: api, id: id),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return server;
}

Finder _action(String label) => find.widgetWithText(OutlinedButton, label);

Future<void> _signOutEverywhere(
  WidgetTester tester, {
  required bool confirm,
}) async {
  await tester.tap(_action(Strings.signOutEverywhere));
  await tester.pumpAndSettle();
  expect(
    find.text(Strings.confirmSignOutUserEverywhere('sam')),
    findsOneWidget,
  );
  await tester.tap(
    confirm
        ? find.widgetWithText(FilledButton, Strings.signOutEverywhere)
        : find.widgetWithText(OutlinedButton, Strings.cancel),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('an admin can reset and sign out another user', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 'u2');

    expect(_action(Strings.setPassword), findsOneWidget);
    expect(_action(Strings.signOutEverywhere), findsOneWidget);
  });

  testWidgets('an admin\'s own page offers neither', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 'u1');

    expect(find.text(Strings.accountProfileHeading), findsOneWidget);
    expect(find.text(Strings.setPassword), findsNothing);
    expect(find.text(Strings.signOutEverywhere), findsNothing);
  });

  testWidgets('a cancelled sign-out sends nothing', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester, 'u2');
    await _signOutEverywhere(tester, confirm: false);

    expect(server.writes, isEmpty);
  });

  testWidgets('signing a user out everywhere keeps the admin on the page', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester, 'u2');
    final int before = server.codeReads;
    await _signOutEverywhere(tester, confirm: true);

    final http.Request write = server.writes.single;
    expect(write.method, 'DELETE');
    expect(write.url.path, '/api/v1/users/u2/sessions');
    expect(find.text(Strings.toastSignedOutEverywhere('sam')), findsOneWidget);
    expect(find.text(Strings.accountProfileHeading), findsOneWidget);
    expect(server.codeReads, before + 1, reason: 'the pending codes are gone');

    await tester.pump(const Duration(seconds: 4));
  });

  testWidgets('setting a password reloads the pending codes', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester, 'u2');
    final int before = server.codeReads;
    await tester.tap(_action(Strings.setPassword));
    await tester.pumpAndSettle();
    await tester.enterText(fieldNamed(Strings.fieldNewPassword), 'hunter2');
    await tester.enterText(fieldNamed(Strings.fieldConfirmPassword), 'hunter2');
    await tester.tap(find.widgetWithText(FilledButton, Strings.setPassword));
    await tester.pumpAndSettle();

    expect(server.writes.single.url.path, '/api/v1/users/u2/password');
    expect(server.codeReads, before + 1);

    await tester.pump(const Duration(seconds: 4));
  });
}
