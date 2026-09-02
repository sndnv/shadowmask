import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/pages/account/library_list_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _episodeCard() => <String, dynamic>{
  'type': 'episode',
  'id': 'e1',
  'season_id': 'se1',
  'number': 3,
  'title': 'The One With The Poster',
  'series_id': 's1',
  'series_title': 'Gamma',
  'season_number': 2,
  'artwork': <String, dynamic>{
    'backdrops': <dynamic>[
      <String, dynamic>{
        'base': '/images/e1-backdrop',
        'widths': <int>[480],
      },
    ],
  },
  'series_artwork': <String, dynamic>{
    'posters': <dynamic>[
      <String, dynamic>{
        'base': '/images/s1-poster',
        'widths': <int>[180],
      },
    ],
  },
};

Future<void> _pump(WidgetTester tester) async {
  tester.view.physicalSize = const Size(1400, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      if (req.url.path == '/api/v1/titles/batch') {
        return http.Response(jsonEncode(<dynamic>[_episodeCard()]), 200);
      }
      return http.Response(jsonEncode(<dynamic>[]), 200);
    }),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: SingleChildScrollView(
              child: LibraryListBlock(
                catalog: CatalogApi(api),
                title: Strings.accountWatchlistHeading,
                emptyMessage: Strings.emptyWatchlist,
                loadRefs: () async => <TitleRef>[
                  const TitleRef(type: TitleKind.episode, id: 'e1'),
                ],
                onRemove: (TitleRef _) async {},
                removeLabel: Strings.removeWatchlist,
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('an episode is a portrait card, not a landscape one', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    final CatalogCardTile tile = tester.widget<CatalogCardTile>(
      find.byType(CatalogCardTile),
    );
    expect(
      tile.card.aspect,
      isNot(CardAspect.landscape),
      reason: 'these lists match history, which shows episodes as posters',
    );
  });

  testWidgets('an episode shows the series poster and names the series', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    final CatalogCardTile tile = tester.widget<CatalogCardTile>(
      find.byType(CatalogCardTile),
    );
    expect(tile.card.title, 'Gamma');
    expect(tile.card.caption, 'The One With The Poster');

    final CardArt art = tester.widget<CardArt>(find.byType(CardArt));
    expect(
      art.artwork?.posters.single.base,
      '/images/s1-poster',
      reason: 'the episode backdrop must not win over the series poster',
    );
  });
}
