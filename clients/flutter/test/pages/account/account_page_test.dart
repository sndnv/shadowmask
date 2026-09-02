import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/account_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

http.Response _route(http.Request req, String role) {
  final String path = req.url.path;
  if (path == '/api/v1/users/self') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'id': 'u1',
        'username': 'pat',
        'role': role,
      }),
      200,
    );
  }
  if (path == '/api/v1/users/u1') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'id': 'u1',
        'username': 'pat',
        'role': role,
        'preferred_audio': <String>[],
        'preferred_subtitle': <String>[],
        'created_at': '2026-01-01T00:00:00Z',
        'updated_at': '2026-01-01T00:00:00Z',
      }),
      200,
    );
  }
  if (path == '/api/v1/users/u1/history') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'items': <dynamic>[],
        'total': 0,
        'offset': 0,
        'limit': 0,
      }),
      200,
    );
  }
  // watchlist, favorites, devices, tokens, link-codes all return empty arrays.
  return http.Response(jsonEncode(<dynamic>[]), 200);
}

Future<void> _pumpAccount(WidgetTester tester, String role) async {
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async => _route(req, role)),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (_) =>
            MaterialPageRoute<void>(builder: (_) => AccountPage(api: api)),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a player sees only Profile and Library tabs', (
    WidgetTester tester,
  ) async {
    await _pumpAccount(tester, 'player');

    expect(find.text('Library'), findsOneWidget);
    expect(find.text('Devices & Access'), findsNothing);
    expect(find.byTooltip(Strings.changePassword), findsNothing);
    expect(find.byTooltip(Strings.editProfile), findsNothing);
    expect(find.text('API tokens'), findsNothing);
    expect(find.text('Sign out everywhere'), findsNothing);

    expect(
      find.text('Watchlist'),
      findsOneWidget,
      reason: 'Library is the landing tab, so the lists need no extra tap',
    );
  });

  testWidgets('a full user sees the Devices & Access tab and management', (
    WidgetTester tester,
  ) async {
    await _pumpAccount(tester, 'user');

    expect(find.text('Devices & Access'), findsOneWidget);

    await tester.tap(find.text('Profile'));
    await tester.pumpAndSettle();
    expect(
      find.widgetWithText(OutlinedButton, Strings.changePassword),
      findsOneWidget,
    );
    expect(
      find.widgetWithText(OutlinedButton, Strings.editProfile),
      findsOneWidget,
    );
    expect(find.text('Sign out everywhere'), findsOneWidget);

    await tester.tap(find.text('Devices & Access'));
    await tester.pumpAndSettle();
    expect(find.text('API tokens'), findsOneWidget);
  });
}
