import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/pagination.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/viewer/collections_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

const int _total = 5;
const int _pageSize = 2;

Map<String, dynamic> _page(int offset) {
  final int end = (offset + _pageSize) > _total ? _total : offset + _pageSize;
  return <String, dynamic>{
    'items': <Map<String, dynamic>>[
      for (int n = offset; n < end; n++)
        <String, dynamic>{
          'id': 'c$n',
          'name': 'Collection $n',
          'artwork': <String, dynamic>{},
        },
    ],
    'total': _total,
    'offset': offset,
    'limit': _pageSize,
  };
}

ApiClient _api(List<int> offsets) => ApiClient(
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
    if (req.url.path == '/api/v1/movies/collections') {
      final int offset =
          int.tryParse(req.url.queryParameters['offset'] ?? '') ?? 0;
      offsets.add(offset);
      return http.Response(jsonEncode(_page(offset)), 200);
    }
    return http.Response(jsonEncode(<dynamic>[]), 200);
  }),
);

Future<void> _pump(WidgetTester tester, ApiClient api) async {
  tester.view.physicalSize = const Size(1400, 1000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (_) =>
            MaterialPageRoute<void>(builder: (_) => CollectionsPage(api: api)),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('collections scroll instead of paging', (
    WidgetTester tester,
  ) async {
    final List<int> offsets = <int>[];
    await _pump(tester, _api(offsets));

    expect(find.byType(Pagination), findsNothing);
    expect(offsets, <int>[0, 2, 4]);
    expect(find.text('Collection 0'), findsOneWidget);
    expect(find.text('Collection 4'), findsOneWidget);
    expect(
      find.text(Strings.countLabel(Strings.navigationCollections, _total)),
      findsOneWidget,
    );
  });
}
