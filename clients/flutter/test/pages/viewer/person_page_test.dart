import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/viewer/person_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _credit(
  String kind,
  String id,
  String title,
  String role, [
  String? character,
]) => <String, dynamic>{
  'title_id': id,
  'kind': kind,
  'display_title': title,
  'year': 2020,
  'artwork': <String, dynamic>{},
  'role': role,
  'character': character,
};

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
    if (req.url.path == '/api/v1/people/p1') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'p1',
          'name': 'Ada Lovelace',
          'artwork': <String, dynamic>{},
          'filmography': <Map<String, dynamic>>[
            _credit('movie', 'm1', 'Alpha', 'actor', 'Hero'),
            _credit('series', 's1', 'Skyline', 'writer'),
            _credit('movie', 'm1', 'Alpha', 'director'),
          ],
        }),
        200,
      );
    }
    return http.Response(jsonEncode(<dynamic>[]), 200);
  }),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a title the person worked on in two roles is one card', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1400, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          onGenerateRoute: (_) => MaterialPageRoute<void>(
            builder: (_) => PersonPage(api: _api(), id: 'p1'),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Alpha'), findsOneWidget);
    expect(
      find.text('${Strings.roleActor} · Hero, ${Strings.roleDirector}'),
      findsOneWidget,
    );
    expect(find.text('Skyline'), findsOneWidget);
    expect(find.text(Strings.roleWriter), findsOneWidget);
  });
}
