import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/pages/viewer/collections_page.dart';
import 'package:shadowmask/pages/viewer/episode_page.dart';
import 'package:shadowmask/pages/viewer/season_page.dart';
import 'package:shadowmask/pages/viewer/title_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/util/page_director.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/backdrops.dart';

const String _before = 'http://test/artwork/before/$kBackdropWidth';

ApiClient _stalled() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) {
    if (req.url.path == '/api/v1/users/self') {
      return Future<http.Response>.value(
        http.Response(
          jsonEncode(<String, dynamic>{
            'id': 'u1',
            'username': 'pat',
            'role': 'user',
          }),
          200,
        ),
      );
    }
    return Completer<http.Response>().future;
  }),
);

Future<PageDirector<String?>> _open(WidgetTester tester, Widget page) async {
  final PageDirector<String?> backdrops = backdropsAfter(_before);
  await tester.pumpWidget(
    BackdropScope(
      backdrops: backdrops,
      child: ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          navigatorObservers: <NavigatorObserver>[backdrops.observer],
          home: page,
        ),
      ),
    ),
  );
  await tester.pump();
  await tester.pump();
  return backdrops;
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a title keeps the backdrop before it while it loads', (
    WidgetTester tester,
  ) async {
    final PageDirector<String?> backdrops = await _open(
      tester,
      TitlePage(api: _stalled(), kind: 'movie', id: 'm1'),
    );

    expect(backdrops.shown, _before);
  });

  testWidgets('a season keeps the backdrop before it while it loads', (
    WidgetTester tester,
  ) async {
    final PageDirector<String?> backdrops = await _open(
      tester,
      SeasonPage(api: _stalled(), id: 'se1'),
    );

    expect(backdrops.shown, _before);
  });

  testWidgets('an episode keeps the backdrop before it while it loads', (
    WidgetTester tester,
  ) async {
    final PageDirector<String?> backdrops = await _open(
      tester,
      EpisodePage(api: _stalled(), id: 'e1'),
    );

    expect(backdrops.shown, _before);
  });

  testWidgets('a collection keeps the backdrop before it while it loads', (
    WidgetTester tester,
  ) async {
    final PageDirector<String?> backdrops = await _open(
      tester,
      CollectionsPage(api: _stalled(), id: 'c1'),
    );

    expect(backdrops.shown, _before);
  });
}
