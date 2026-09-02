import 'dart:convert';

import 'package:fluro/fluro.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/app_router.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/entry/not_found_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

ApiClient _api() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
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
    return http.Response(jsonEncode(<dynamic>[]), 200);
  }),
);

Future<void> _pump(WidgetTester tester, {String? path}) async {
  tester.view.physicalSize = const Size(1200, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: NotFoundPage(api: _api(), path: path),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a stale link says so instead of asking for credentials', (
    WidgetTester tester,
  ) async {
    await _pump(tester, path: '/tilte?id=m1');

    expect(find.text(Strings.notFoundBody), findsOneWidget);
    expect(
      find.text('/tilte?id=m1'),
      findsOneWidget,
      reason: 'naming the address is what tells you the link was wrong',
    );
    expect(
      find.text(Strings.signInTitle),
      findsNothing,
      reason: 'a bad link used to land on a credential form',
    );
  });

  testWidgets('it offers the way back', (WidgetTester tester) async {
    await _pump(tester, path: '/nope');

    expect(find.text(Strings.navigationHome), findsWidgets);
  });

  test('an unknown route resolves to the not-found page', () {
    final AppRouter router = AppRouter(_api());
    final Handler? handler = router.router.notFoundHandler;

    expect(
      handler?.handlerFunc(null, <String, List<String>>{}),
      isA<NotFoundPage>(),
    );
  });
}
