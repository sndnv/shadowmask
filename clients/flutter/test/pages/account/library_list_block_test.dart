import 'dart:convert';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/card_menu.dart';
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

final List<String> _removed = <String>[];
final List<String> _onList = <String>['e1'];

class _Revised extends StatefulWidget {
  const _Revised({required this.builder});

  final Widget Function(int revision, VoidCallback onChanged) builder;

  @override
  State<_Revised> createState() => _RevisedState();
}

class _RevisedState extends State<_Revised> {
  int _revision = 0;

  @override
  Widget build(BuildContext context) =>
      widget.builder(_revision, () => setState(() => _revision += 1));
}

Widget _hosted({
  required bool menued,
  required CatalogApi catalog,
  required Widget child,
}) =>
    menued ? CardMenuHost(catalog: catalog, userId: 'u1', child: child) : child;

Future<void> _pump(WidgetTester tester, {bool menued = false}) async {
  tester.view.physicalSize = const Size(1400, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      if (req.url.path == '/api/v1/titles/batch') {
        return http.Response(jsonEncode(<dynamic>[_episodeCard()]), 200);
      }
      if (req.method == 'DELETE' && req.url.path.contains('/watchlist/')) {
        final String id = req.url.path.split('/watchlist/').last;
        _removed.add(id);
        _onList.remove(id);
        return http.Response('', 204);
      }
      if (req.url.path == '/api/v1/users/u1/state/batch') {
        return http.Response(
          jsonEncode(<dynamic>[
            <String, dynamic>{
              'title': <String, dynamic>{'type': 'episode', 'id': 'e1'},
              'watchlisted': true,
            },
          ]),
          200,
        );
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
              child: _hosted(
                menued: menued,
                catalog: CatalogApi(api),
                child: _Revised(
                  builder: (int revision, VoidCallback onChanged) =>
                      LibraryListBlock(
                        catalog: CatalogApi(api),
                        title: Strings.accountWatchlistHeading,
                        emptyMessage: Strings.emptyWatchlist,
                        loadRefs: () async => <TitleRef>[
                          for (final String id in _onList)
                            TitleRef(type: TitleKind.episode, id: id),
                        ],
                        onRemove: (TitleRef _) async {},
                        removeLabel: Strings.removeWatchlist,
                        revision: revision,
                        onChanged: onChanged,
                      ),
                ),
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
  setUp(() {
    SharedPreferences.setMockInitialValues(<String, Object>{});
    _removed.clear();
    _onList
      ..clear()
      ..add('e1');
  });

  testWidgets('a card taken off the list by the menu leaves the block', (
    WidgetTester tester,
  ) async {
    await _pump(tester, menued: true);

    await tester.tap(find.byType(CatalogCardTile), buttons: kSecondaryButton);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.removeWatchlist).last);
    await tester.pumpAndSettle();

    expect(_removed, <String>['e1']);
    expect(
      find.byType(CatalogCardTile),
      findsNothing,
      reason: 'it is no longer on the watchlist this block is showing',
    );
  });

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
