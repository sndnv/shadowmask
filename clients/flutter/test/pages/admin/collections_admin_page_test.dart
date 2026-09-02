import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/collections_admin_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

http.Response _route(http.Request req) {
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
  if (req.url.path == '/api/v1/search') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'items': <dynamic>[
          <String, dynamic>{
            'type': 'movie',
            'id': 'm1',
            'title': 'The Thing',
            'year': 1982,
          },
        ],
        'total': 1,
        'offset': 0,
        'limit': 20,
      }),
      200,
    );
  }
  if (req.url.path == '/api/v1/movies/collections') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'items': <dynamic>[],
        'total': 0,
        'offset': 0,
        'limit': 20,
      }),
      200,
    );
  }
  return http.Response('{}', 200);
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a member search result is one clickable line that adds it', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1400, 1200);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async => _route(req)),
    );

    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          builder: (BuildContext context, Widget? child) =>
              ToastHost(child: child ?? const SizedBox.shrink()),
          onGenerateRoute: (_) => MaterialPageRoute<void>(
            builder: (_) => CollectionsAdminPage(api: api),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.widgetWithText(FilledButton, Strings.createCollection),
    );
    await tester.pumpAndSettle();

    await tester.enterText(fieldNamed(Strings.fieldSearchQuery), 'thing');
    await tester.tap(find.widgetWithText(FilledButton, Strings.searchAction));
    await tester.pumpAndSettle();

    expect(find.text('The Thing (1982)'), findsOneWidget);

    await tester.tap(find.text('The Thing (1982)'));
    await tester.pumpAndSettle();

    expect(find.widgetWithText(InputChip, 'The Thing'), findsOneWidget);
  });
}
