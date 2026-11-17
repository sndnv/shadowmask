import 'dart:convert';

import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/crumb_strip.dart';
import 'package:shadowmask/components/tab_heading.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/dashboard_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/shape_guard.dart';

Future<void> _pump(
  WidgetTester tester, {
  AppThemeVariant variant = AppThemeVariant.dark,
}) async {
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
      variant: variant,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(variant),
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

  for (final AppThemeVariant variant in AppThemeVariant.values) {
    testWidgets('the dashboard draws the ${variant.name} shapes', (
      WidgetTester tester,
    ) async {
      await _pump(tester, variant: variant);

      expectShapesOf(tester, variant);
    });
  }

  testWidgets('hologram heads each group with a tab', (
    WidgetTester tester,
  ) async {
    await _pump(tester, variant: AppThemeVariant.hologram);

    expect(find.byType(TabHeading), findsNWidgets(2));
    expect(
      tester
          .widget<Text>(
            find.descendant(
              of: find.byType(CrumbStrip),
              matching: find.text(Strings.adminHeading),
            ),
          )
          .style
          ?.fontFamily,
      kMonoFamily,
    );
    expect(find.text(Strings.adminGroupContent.toUpperCase()), findsOneWidget);
    expect(
      find.text(Strings.adminGroupOperations.toUpperCase()),
      findsOneWidget,
    );
  });

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

  testWidgets('the Titles card opens the title list', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text(Strings.adminTitlesAbout), findsOneWidget);

    await tester.tap(find.text(Strings.adminTitles));
    await tester.pumpAndSettle();

    expect(find.text('went to /admin/titles'), findsOneWidget);
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
