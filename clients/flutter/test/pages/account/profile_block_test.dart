import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/pages/account/profile_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Future<void> _pump(
  WidgetTester tester, {
  required List<String> audio,
  required List<String> subtitle,
}) async {
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient(
      (http.Request _) async => http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': 'user',
          'preferred_audio': audio,
          'preferred_subtitle': subtitle,
          'created_at': '2026-01-01T00:00:00Z',
          'updated_at': '2026-01-01T00:00:00Z',
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
        home: Scaffold(
          body: SingleChildScrollView(
            child: ProfileBlock(api: api, userId: 'u1'),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('preferred languages read as names, not codes', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      audio: <String>['en', 'pt-BR'],
      subtitle: <String>['ja'],
    );

    expect(find.text('English, Portuguese (Brazil)'), findsOneWidget);
    expect(find.text('Japanese'), findsOneWidget);
    expect(find.text('en, pt-BR'), findsNothing);
    expect(find.text('ja'), findsNothing);
  });
}
