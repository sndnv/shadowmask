import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/random_button.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/random_pick.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

ApiClient _api(List<String> seen, {int status = 200, String? code}) =>
    ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        seen.add('${req.method} ${req.url.path}?${req.url.query}');
        if (status != 200) {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'error': <String, dynamic>{
                'code': code,
                'message': 'nothing to pick',
              },
            }),
            status,
          );
        }
        return http.Response(
          jsonEncode(<String, dynamic>{'version_id': 'v-picked'}),
          200,
        );
      }),
    );

Future<List<String>> _pump(
  WidgetTester tester,
  ApiClient api, {
  Future<RandomPick> Function(CatalogApi)? pick,
}) async {
  final List<String> routes = <String>[];
  final CatalogApi catalog = CatalogApi(api);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (RouteSettings settings) {
          routes.add(settings.name ?? '');
          return MaterialPageRoute<void>(
            builder: (_) => const SizedBox.shrink(),
            settings: settings,
          );
        },
        home: ToastHost(
          child: Scaffold(
            body: Center(
              child: RandomButton(
                tooltip: Strings.randomMovie,
                pick: () =>
                    (pick ?? (CatalogApi c) => c.randomMovie())(catalog),
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return routes;
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a pick sends the viewer straight to the watch page', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    final List<String> routes = await _pump(tester, _api(seen));

    await tester.tap(find.byType(RandomButton));
    await tester.pumpAndSettle();

    expect(seen, <String>['GET /api/v1/movies/random?']);
    expect(
      routes.last,
      '/watch?version=v-picked',
      reason: 'random play must start playing, not open a picker',
    );
  });

  testWidgets('the active filters travel with the pick', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(
      tester,
      _api(seen),
      pick: (CatalogApi c) =>
          c.randomMovie(genres: <String>['Horror', 'Drama'], library: 'lib-4k'),
    );

    await tester.tap(find.byType(RandomButton));
    await tester.pumpAndSettle();

    expect(seen.single, contains('genres=Horror%2CDrama'));
    expect(seen.single, contains('library=lib-4k'));
  });

  testWidgets(
    'an empty pool says so plainly instead of it is no longer there',
    (WidgetTester tester) async {
      final List<String> seen = <String>[];
      await _pump(tester, _api(seen, status: 404, code: 'not_found'));

      await tester.tap(find.byType(RandomButton));
      await tester.pumpAndSettle();

      expect(find.text(Strings.randomNothingToPlay), findsOneWidget);
      expect(find.textContaining(Strings.reasonNotFound), findsNothing);
      await tester.pump(
        kErrorToastDuration + const Duration(milliseconds: 100),
      );
    },
  );

  testWidgets('any other failure surfaces the reason', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(tester, _api(seen, status: 500));

    await tester.tap(find.byType(RandomButton));
    await tester.pumpAndSettle();

    expect(find.textContaining(Strings.errorRandom), findsOneWidget);
    await tester.pump(kErrorToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a failed pick leaves the button usable again', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(tester, _api(seen, status: 500));

    await tester.tap(find.byType(RandomButton));
    await tester.pumpAndSettle();
    await tester.tap(find.byType(RandomButton));
    await tester.pumpAndSettle();

    expect(seen.length, 2, reason: 'the button must not stay stuck busy');
    await tester.pump(kErrorToastDuration + const Duration(milliseconds: 100));
  });
}
