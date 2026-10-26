import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/profile_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

class _Server {
  _Server({this.failWith});

  final String? failWith;
  final List<http.Request> writes = <http.Request>[];

  http.Response answer(http.Request req) {
    if (req.method == 'GET' && req.url.path == '/api/v1/users/u2') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u2',
          'username': 'sam',
          'role': 'user',
          'preferred_audio': <String>[],
          'preferred_subtitle': <String>[],
          'created_at': '2026-01-01T00:00:00Z',
          'updated_at': '2026-01-01T00:00:00Z',
        }),
        200,
      );
    }
    writes.add(req);
    final String? code = failWith;
    if (code != null) {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'error': <String, dynamic>{'code': code, 'message': 'refused'},
        }),
        400,
      );
    }
    return http.Response('', 204);
  }
}

Future<_Server> _pump(WidgetTester tester, {String? failWith}) async {
  tester.view.physicalSize = const Size(1600, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final _Server server = _Server(failWith: failWith);
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
            child: ProfileBlock(api: api, userId: 'u2', showAdminActions: true),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  await tester.tap(find.widgetWithText(OutlinedButton, Strings.setPassword));
  await tester.pumpAndSettle();
  return server;
}

Future<void> _submit(WidgetTester tester, String next, String confirm) async {
  await tester.enterText(fieldNamed(Strings.fieldNewPassword), next);
  await tester.enterText(fieldNamed(Strings.fieldConfirmPassword), confirm);
  await tester.tap(find.widgetWithText(FilledButton, Strings.setPassword));
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the dialog names the user and asks for no current password', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text(Strings.setPasswordTitle('sam')), findsOneWidget);
    expect(fieldNamed(Strings.fieldCurrentPassword), findsNothing);
  });

  testWidgets('an empty new password is rejected', (WidgetTester tester) async {
    final _Server server = await _pump(tester);
    await _submit(tester, '', '');

    expect(find.text(Strings.requiredNewPassword), findsOneWidget);
    expect(server.writes, isEmpty);
  });

  testWidgets('a mismatched confirmation is rejected', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester);
    await _submit(tester, 'hunter2', 'hunter3');

    expect(find.text(Strings.passwordsDoNotMatch), findsOneWidget);
    expect(server.writes, isEmpty);
  });

  testWidgets('setting a password sends only the new one and keeps the admin', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester);
    await _submit(tester, 'hunter2', 'hunter2');

    final http.Request write = server.writes.single;
    expect(write.method, 'PUT');
    expect(write.url.path, '/api/v1/users/u2/password');
    expect(jsonDecode(write.body), <String, dynamic>{
      'new_password': 'hunter2',
    });
    expect(find.text(Strings.setPasswordTitle('sam')), findsNothing);
    expect(find.text(Strings.toastPasswordSet('sam')), findsOneWidget);
    expect(find.text(Strings.accountProfileHeading), findsOneWidget);

    await tester.pump(const Duration(seconds: 4));
  });

  testWidgets('a refused password keeps the dialog open', (
    WidgetTester tester,
  ) async {
    await _pump(tester, failWith: 'empty_password');
    await _submit(tester, ' ', ' ');

    expect(
      find.textContaining(Strings.errorSetPassword, findRichText: true),
      findsOneWidget,
    );
    expect(find.text(Strings.setPasswordTitle('sam')), findsOneWidget);

    await tester.pump(const Duration(seconds: 4));
  });
}
