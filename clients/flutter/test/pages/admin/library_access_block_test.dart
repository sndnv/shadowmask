import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/pages/admin/library_access_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _library(String id, String name) => <String, dynamic>{
  'id': id,
  'name': name,
  'kind': 'movie',
  'origin': 'local',
  'roots': <String>[],
  'watcher': 'manual',
  'metadata_sources': <String>[],
  'sort_articles': <String>[],
  'created_at': '2026-08-17T07:24:11Z',
  'updated_at': '2026-08-17T07:24:11Z',
};

ApiClient _api() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    if (req.url.path == '/api/v1/libraries') {
      return http.Response(
        jsonEncode(<Map<String, dynamic>>[
          _library('l1', 'Films'),
          _library('l2', 'Shows'),
        ]),
        200,
        headers: <String, String>{'content-type': 'application/json'},
      );
    }
    if (req.url.path == '/api/v1/users/u1/libraries') {
      return http.Response(
        jsonEncode(<Map<String, dynamic>>[
          <String, dynamic>{'library_id': 'l1'},
        ]),
        200,
        headers: <String, String>{'content-type': 'application/json'},
      );
    }
    return http.Response('{}', 404);
  }),
);

Future<void> _pump(WidgetTester tester) async {
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        home: Scaffold(
          body: LibraryAccessBlock(api: _api(), userId: 'u1'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the granted libraries are ticked inside the framed section', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(tester.takeException(), isNull);
    expect(find.byType(CheckboxListTile), findsNWidgets(2));
    expect(
      tester
          .widget<CheckboxListTile>(find.byType(CheckboxListTile).first)
          .value,
      isTrue,
    );
    expect(
      tester.widget<CheckboxListTile>(find.byType(CheckboxListTile).last).value,
      isFalse,
    );
  });

  testWidgets('the tiles get their own material so the ink stays visible', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    final Finder nearest = find
        .ancestor(
          of: find.byType(CheckboxListTile).first,
          matching: find.byType(Material),
        )
        .first;
    expect(tester.widget<Material>(nearest).type, MaterialType.transparency);
  });
}
