import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/cast_rail.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Credit _credit(String id, String name, CreditRole role, int order) => Credit(
  person: PersonRef(id: id, name: name),
  role: role,
  order: order,
);

ApiClient _api({List<String>? seen, bool fail = false}) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    seen?.add(req.url.path);
    if (fail) {
      return http.Response('{}', 500);
    }
    return http.Response(
      jsonEncode(<Map<String, dynamic>>[
        <String, dynamic>{
          'id': 'p1',
          'name': 'Amy Adams',
          'artwork': <String, dynamic>{
            'posters': <dynamic>[
              <String, dynamic>{
                'base': '/images/p1',
                'widths': <int>[480],
              },
            ],
          },
        },
      ]),
      200,
    );
  }),
);

Future<void> _pump(WidgetTester tester, ApiClient api, List<Credit> credits) =>
    tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: Scaffold(
            body: CastRail(catalog: CatalogApi(api), credits: credits),
          ),
        ),
      ),
    );

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('billed cast prefers actors, orders them and drops duplicates', () {
    final List<Credit> credits = <Credit>[
      _credit('p2', 'Jeremy Renner', CreditRole.actor, 2),
      _credit('p9', 'Denis Villeneuve', CreditRole.director, 0),
      _credit('p1', 'Amy Adams', CreditRole.actor, 1),
      _credit('p1', 'Amy Adams', CreditRole.actor, 5),
    ];

    final List<Credit> billed = billedCast(credits);
    expect(
      billed.map((Credit c) => c.person.id).toList(),
      <String>['p1', 'p2'],
      reason: 'directors are dropped when there are actors, and ids are unique',
    );
  });

  test('with no actors at all the other credits are used', () {
    final List<Credit> billed = billedCast(<Credit>[
      _credit('p9', 'Denis Villeneuve', CreditRole.director, 0),
    ]);
    expect(billed.single.person.id, 'p9');
  });

  testWidgets('names render before the photos arrive', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(), <Credit>[
      _credit('p1', 'Amy Adams', CreditRole.actor, 0),
    ]);
    await tester.pump();

    expect(find.text('Amy Adams'), findsOneWidget);

    await tester.pumpAndSettle();
    expect(find.text('Amy Adams'), findsOneWidget);
  });

  testWidgets('a failed photo fetch leaves the names in place', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(fail: true), <Credit>[
      _credit('p1', 'Amy Adams', CreditRole.actor, 0),
    ]);
    await tester.pumpAndSettle();

    expect(find.text('Amy Adams'), findsOneWidget);
  });

  testWidgets('photos are fetched in one batch, not one call per person', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(tester, _api(seen: seen), <Credit>[
      _credit('p1', 'Amy Adams', CreditRole.actor, 0),
      _credit('p2', 'Jeremy Renner', CreditRole.actor, 1),
      _credit('p3', 'Forest Whitaker', CreditRole.actor, 2),
    ]);
    await tester.pumpAndSettle();

    expect(seen, <String>['/api/v1/people/batch']);
  });

  testWidgets('no credits means no rail', (WidgetTester tester) async {
    final List<String> seen = <String>[];
    await _pump(tester, _api(seen: seen), <Credit>[]);
    await tester.pumpAndSettle();

    expect(find.byType(CastRail), findsOneWidget);
    expect(seen, isEmpty);
  });
}
