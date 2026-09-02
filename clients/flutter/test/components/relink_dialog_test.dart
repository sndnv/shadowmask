import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/admin/relink_dialog.dart';
import 'package:shadowmask/components/segmented_tabs.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _hit(String id, String title) => <String, dynamic>{
  'ref': <String, dynamic>{'type': 'movie', 'id': id},
  'type': 'movie',
  'id': id,
  'title': title,
  'year': 1982,
};

Finder _tab(String label) => find.descendant(
  of: find.byType(SegmentedTabs<RelinkMode>),
  matching: find.text(label),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('picking a match relinks every version to it', (
    WidgetTester tester,
  ) async {
    final List<String> relinked = <String>[];
    Map<String, dynamic>? sentTarget;
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        if (req.url.path == '/api/v1/search') {
          expect(req.url.queryParameters['type'], 'movie');
          return http.Response(
            jsonEncode(<String, dynamic>{
              'items': <dynamic>[_hit('m1', 'The Thing')],
              'total': 1,
              'offset': 0,
              'limit': 20,
            }),
            200,
          );
        }
        if (req.url.path.endsWith('/relink')) {
          relinked.add(req.url.pathSegments[req.url.pathSegments.length - 2]);
          sentTarget =
              (jsonDecode(req.body) as Map<String, dynamic>)['target']
                  as Map<String, dynamic>;
          return http.Response('', 204);
        }
        return http.Response('{}', 200);
      }),
    );

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        home: Scaffold(
          body: RelinkDialog(
            admin: AdminApi(api),
            catalog: CatalogApi(api),
            versionIds: const <String>['v1', 'v2'],
          ),
        ),
      ),
    );

    await tester.tap(_tab(Strings.tabMove));
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField).first, 'thing');
    await tester.testTextInput.receiveAction(TextInputAction.search);
    await tester.pumpAndSettle();

    expect(find.text('The Thing (1982)'), findsOneWidget);

    await tester.tap(find.text('The Thing (1982)'));
    await tester.pumpAndSettle();

    expect(find.text(Strings.confirmMoveHeading), findsOneWidget);
    expect(relinked, isEmpty);

    await tester.tap(find.widgetWithText(FilledButton, Strings.tabMove));
    await tester.pumpAndSettle();

    expect(relinked, <String>['v1', 'v2']);
    expect(sentTarget?['kind'], 'existing');
    expect((sentTarget?['title'] as Map<String, dynamic>)['id'], 'm1');

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('the Relink tab sends a tmdb id with no provider field', (
    WidgetTester tester,
  ) async {
    Map<String, dynamic>? sentTarget;
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        if (req.url.path.endsWith('/relink')) {
          sentTarget =
              (jsonDecode(req.body) as Map<String, dynamic>)['target']
                  as Map<String, dynamic>;
          return http.Response('', 204);
        }
        return http.Response('{}', 200);
      }),
    );

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        home: Scaffold(
          body: RelinkDialog(
            admin: AdminApi(api),
            catalog: CatalogApi(api),
            versionIds: const <String>['v1'],
          ),
        ),
      ),
    );

    await tester.tap(_tab(Strings.tabRelink));
    await tester.pumpAndSettle();

    expect(find.text(Strings.fieldProviderSource), findsNothing);
    expect(find.byType(TextField), findsOneWidget);

    await tester.enterText(find.byType(TextField), 'movie/603');
    await tester.tap(find.byIcon(Icons.link));
    await tester.pumpAndSettle();

    expect(find.text(Strings.confirmRelinkBody('movie/603')), findsOneWidget);
    expect(sentTarget, isNull);

    await tester.tap(find.widgetWithText(FilledButton, Strings.relink));
    await tester.pumpAndSettle();

    expect(sentTarget?['kind'], 'provider');
    expect(sentTarget?['source'], 'tmdb');
    expect(sentTarget?['value'], 'movie/603');

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a search with no hits reports on the same inline line', (
    WidgetTester tester,
  ) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient(
        (http.Request _) async => http.Response(
          jsonEncode(<String, dynamic>{
            'items': <dynamic>[],
            'total': 0,
            'offset': 0,
            'limit': 20,
          }),
          200,
        ),
      ),
    );

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        home: Scaffold(
          body: RelinkDialog(
            admin: AdminApi(api),
            catalog: CatalogApi(api),
            versionIds: const <String>['v1'],
          ),
        ),
      ),
    );

    await tester.tap(_tab(Strings.tabMove));
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField).first, 'nothing');
    await tester.tap(find.byIcon(Icons.search));
    await tester.pumpAndSettle();

    expect(find.text(Strings.noMatches), findsOneWidget);
  });

  testWidgets('an empty input reports inline, not as a toast', (
    WidgetTester tester,
  ) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient(
        (http.Request _) async => http.Response('{}', 200),
      ),
    );

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        home: Scaffold(
          body: RelinkDialog(
            admin: AdminApi(api),
            catalog: CatalogApi(api),
            versionIds: const <String>['v1'],
          ),
        ),
      ),
    );

    await tester.tap(_tab(Strings.tabMove));
    await tester.pumpAndSettle();

    await tester.testTextInput.receiveAction(TextInputAction.search);
    await tester.pumpAndSettle();
    expect(find.text(Strings.requiredSearch), findsOneWidget);

    await tester.tap(_tab(Strings.tabRelink));
    await tester.pumpAndSettle();
    expect(find.text(Strings.requiredSearch), findsNothing);

    await tester.tap(find.byIcon(Icons.link));
    await tester.pumpAndSettle();
    expect(find.text(Strings.requiredTmdbId), findsOneWidget);
  });
}
