import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/dashboard_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Future<void> _pump(WidgetTester tester) async {
  tester.view.physicalSize = const Size(1600, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient(
      (http.Request req) async => http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': 'admin',
        }),
        200,
      ),
    ),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) => settings.name == null || settings.name == '/'
              ? DashboardPage(api: api)
              : Text('went to ${settings.name}'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the Server card opens the server page', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text('Debugging and diagnostics.'), findsOneWidget);

    await tester.tap(find.text('Server'));
    await tester.pumpAndSettle();

    expect(find.text('went to /admin/server'), findsOneWidget);
    expect(find.text('Server logs'), findsNothing);
  });

  testWidgets('the Reports card opens the reports', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text(Strings.adminReportsAbout), findsOneWidget);

    await tester.tap(find.text(Strings.adminReports));
    await tester.pumpAndSettle();

    expect(find.text('went to /admin/reports'), findsOneWidget);
  });
}
