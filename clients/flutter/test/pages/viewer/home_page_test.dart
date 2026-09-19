import 'dart:convert';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/viewer/home_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _episode(String id) => <String, dynamic>{
  'type': 'episode',
  'id': id,
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

Map<String, dynamic> _movie(String id) => <String, dynamic>{
  'type': 'movie',
  'id': id,
  'title': 'Alpha',
  'year': 2021,
};

List<dynamic> _hubs({required bool watchlist}) => <dynamic>[
  if (watchlist || _added.isNotEmpty)
    <String, dynamic>{
      'id': 'watchlist',
      'title': 'On Your Watchlist',
      'items': <dynamic>[
        if (watchlist && !_removed.contains('e1')) _episode('e1'),
        if (watchlist && !_removed.contains('m2')) _movie('m2'),
        for (final String id in _added) _movie(id),
      ],
    },
  <String, dynamic>{
    'id': 'recently_added_movies',
    'title': 'Recently Added Movies',
    'items': <dynamic>[_movie('m1')],
  },
];

Map<String, dynamic> _upNext() => <String, dynamic>{
  'next_movies': <dynamic>[_movie('m9')],
};

Map<String, dynamic> _inProgress() => <String, dynamic>{
  'in_progress': <dynamic>[
    <String, dynamic>{
      'card': <String, dynamic>{
        'title': <String, dynamic>{'type': 'movie', 'id': 'm9'},
        'display_title': 'Delta',
        'progress_percent': 40,
      },
      'progress': <String, dynamic>{'version_id': 'v9'},
    },
  ],
};

final List<String> _removed = <String>[];
final List<String> _cleared = <String>[];
final List<String> _added = <String>[];
final Set<String> _watched = <String>{};
int _hubCalls = 0;
String? _pushedRoute;

Future<void> _pump(
  WidgetTester tester, {
  required bool watchlist,
  bool? watchlistOnReload,
  bool hubFails = false,
  bool recoverOnRetry = false,
  bool emptyLibrary = false,
  bool admin = false,
  bool sharedLibraries = false,
  bool resuming = false,
  bool upNext = false,
}) async {
  tester.view.physicalSize = const Size(1400, 1600);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      final String path = req.url.path;
      if (req.method == 'DELETE' && path.contains('/watchlist/')) {
        _removed.add(path.split('/watchlist/').last);
        return http.Response('', 204);
      }
      if (path == '/api/v1/users/self') {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'id': 'u1',
            'username': 'pat',
            'role': admin ? 'admin' : 'user',
          }),
          200,
        );
      }
      if (path == '/api/v1/libraries') {
        return http.Response(
          jsonEncode(<dynamic>[
            if (sharedLibraries) <String, dynamic>{'id': 'l1', 'name': 'Films'},
          ]),
          200,
        );
      }
      if (path == '/api/v1/users/u1/hub') {
        _hubCalls++;
        if (hubFails && (_hubCalls == 1 || !recoverOnRetry)) {
          return http.Response('{"message":"prefs.db is unreadable"}', 500);
        }
        if (emptyLibrary) {
          return http.Response(jsonEncode(<dynamic>[]), 200);
        }
        final bool saved = _hubCalls > 1
            ? (watchlistOnReload ?? watchlist)
            : watchlist;
        return http.Response(jsonEncode(_hubs(watchlist: saved)), 200);
      }
      if (path == '/api/v1/users/u1/state/batch') {
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
      if (req.method == 'PUT' && path.contains('/watchlist/')) {
        _added.add(path.split('/watchlist/').last);
        return http.Response('', 204);
      }
      if (req.method == 'PUT' && path.contains('/watched/')) {
        _watched.add(path.split('/watched/').last);
        return http.Response('', 204);
      }
      if (req.method == 'DELETE' && path.contains('/progress/')) {
        _cleared.add(path.split('/progress/').last);
        return http.Response('', 204);
      }
      if (path == '/api/v1/users/u1/continue') {
        return http.Response(
          jsonEncode(<String, dynamic>{
            if (resuming) ..._inProgress(),
            if (upNext && !_watched.contains('m9')) ..._upNext(),
          }),
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
        navigatorObservers: <NavigatorObserver>[appRouteObserver],
        onGenerateRoute: (RouteSettings s) {
          if (s.name != null && s.name != '/') {
            _pushedRoute = s.name;
          }
          return MaterialPageRoute<void>(builder: (_) => HomePage(api: api));
        },
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() {
    SharedPreferences.setMockInitialValues(<String, Object>{});
    _removed.clear();
    _cleared.clear();
    _added.clear();
    _watched.clear();
    _hubCalls = 0;
    _pushedRoute = null;
  });

  testWidgets('marking an Up next title watched takes it off the rail', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: false, upNext: true);
    expect(find.text(Strings.upNext), findsOneWidget);

    await tester.tap(
      find.byType(CatalogCardTile).first,
      buttons: kSecondaryButton,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.markWatched));
    await tester.pumpAndSettle();

    expect(_watched, <String>{'m9'});
    expect(
      find.text(Strings.upNext),
      findsNothing,
      reason: 'a watched title is not what you watch next',
    );
  });

  testWidgets('saving a title from another rail fills the watchlist rail', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: false);
    expect(find.text(Strings.onYourWatchlist), findsNothing);

    await tester.tap(
      find.byType(CatalogCardTile).first,
      buttons: kSecondaryButton,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.addWatchlist));
    await tester.pumpAndSettle();

    expect(_added, <String>['m1']);
    expect(
      find.text(Strings.onYourWatchlist),
      findsOneWidget,
      reason: 'the rail it was just added to has to show it',
    );
  });

  testWidgets('the page never empties while it re-reads itself', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: false);

    await tester.tap(
      find.byType(CatalogCardTile).first,
      buttons: kSecondaryButton,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.addWatchlist));

    for (int frame = 0; frame < 12; frame++) {
      await tester.pump(const Duration(milliseconds: 16));
      expect(
        find.byType(CardRail),
        findsWidgets,
        reason: 'frame $frame went blank; the reload must not flash a skeleton',
      );
    }
    await tester.pumpAndSettle();
  });

  testWidgets('the menu removal drops the card the X button would drop', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: true);

    await tester.tap(
      find.byType(CatalogCardTile).first,
      buttons: kSecondaryButton,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.removeWatchlist));
    await tester.pumpAndSettle();

    expect(_removed, <String>['e1'], reason: 'the call itself goes out');
    final CardRail rail = tester.widget<CardRail>(find.byType(CardRail).first);
    expect(
      rail.cards.length,
      1,
      reason: 'the X button drops it from the rail, the menu must too',
    );
  });

  testWidgets('the Continue watching removal empties the rail from the menu', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: false, resuming: true);
    expect(find.text(Strings.continueWatching), findsOneWidget);

    await tester.tap(
      find.byType(CatalogCardTile).first,
      buttons: kSecondaryButton,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.dismissResume));
    await tester.pumpAndSettle();

    expect(_cleared, <String>['v9']);
    expect(
      find.text(Strings.continueWatching),
      findsNothing,
      reason: 'the only card went, so the rail should go with it',
    );
  });

  testWidgets('the watchlist rail leads the home page', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: true);

    final List<String> rails = tester
        .widgetList<CardRail>(find.byType(CardRail))
        .map((CardRail r) => r.title)
        .toList();
    expect(rails, <String>[
      Strings.onYourWatchlist,
      Strings.recentlyAddedMovies,
    ]);
  });

  testWidgets('a saved episode keeps its own card and the series poster', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: true);

    final CardRail rail = tester.widget<CardRail>(find.byType(CardRail).first);
    expect(
      rail.cards.length,
      2,
      reason: 'one card per saved row, never folded',
    );
    final CatalogCard episode = rail.cards.first;
    expect(episode.ref.type, TitleKind.episode);
    expect(episode.title, 'Gamma');
    expect(episode.artwork?.posters.single.base, '/images/s1-poster');
  });

  testWidgets('dismissing a watchlist card removes that one saved row', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: true);

    await tester.tap(find.byTooltip(Strings.removeWatchlist).first);
    await tester.pumpAndSettle();

    expect(_removed, <String>[
      'e1',
    ], reason: 'the episode goes, the movie stays');
    final CardRail rail = tester.widget<CardRail>(find.byType(CardRail).first);
    expect(rail.cards.length, 1);
    expect(rail.cards.single.ref.id, 'm2');
  });

  testWidgets('the recently added rail offers no dismiss control', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: false);

    expect(find.byTooltip(Strings.removeWatchlist), findsNothing);
  });

  testWidgets('no watchlist hub means no rail at all', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: false);

    final List<String> rails = tester
        .widgetList<CardRail>(find.byType(CardRail))
        .map((CardRail r) => r.title)
        .toList();
    expect(rails, <String>[Strings.recentlyAddedMovies]);
  });

  testWidgets('a failing hub reads as a failure, not an empty library', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: true, hubFails: true);

    expect(
      find.text(Strings.couldNotLoadHome),
      findsOneWidget,
      reason:
          'one unreadable prefs.db blanks every rail, and calling that an empty library sends the '
          'user looking for a library problem that does not exist',
    );
    expect(find.text(Strings.nothingToShowYet), findsNothing);
    expect(find.byType(CardRail), findsNothing);
  });

  testWidgets('a failed home offers a retry that works', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: true, hubFails: true, recoverOnRetry: true);

    expect(find.widgetWithText(OutlinedButton, Strings.retry), findsOneWidget);

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.retry));
    await tester.pumpAndSettle();

    expect(find.text(Strings.couldNotLoadHome), findsNothing);
    expect(find.byType(CardRail), findsWidgets);
  });

  testWidgets('a genuinely empty library still says so', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      watchlist: false,
      emptyLibrary: true,
      sharedLibraries: true,
    );

    expect(
      find.text(Strings.nothingToShowYet),
      findsOneWidget,
      reason: 'loaded and empty is not the same as failed to load',
    );
    expect(find.text(Strings.couldNotLoadHome), findsNothing);
  });

  testWidgets('a viewer with no shared library is told to ask an admin', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: false, emptyLibrary: true);

    expect(find.text(Strings.noLibrariesShared), findsOneWidget);
    expect(
      find.text(Strings.nothingToShowYet),
      findsNothing,
      reason: 'nothing exists and nothing is shared with you are not the same',
    );
    expect(find.text(Strings.createLibrary), findsNothing);
  });

  testWidgets('an admin with no libraries is pointed at creating one', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: false, emptyLibrary: true, admin: true);

    expect(find.text(Strings.emptyLibraries), findsOneWidget);
    expect(find.text(Strings.createLibrary), findsOneWidget);

    await tester.tap(find.text(Strings.createLibrary));
    await tester.pumpAndSettle();

    expect(
      _pushedRoute,
      adminLibrariesRoute(),
      reason: 'the empty line has to land on the page that fixes it',
    );
  });

  testWidgets('coming back to home refetches the rails', (
    WidgetTester tester,
  ) async {
    await _pump(tester, watchlist: true, watchlistOnReload: false);
    expect(_hubCalls, 1);

    final NavigatorState nav = tester.state<NavigatorState>(
      find.byType(Navigator),
    );
    nav.push(
      MaterialPageRoute<void>(
        builder: (_) => const Scaffold(body: Text('a title page')),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('a title page'), findsOneWidget);

    nav.pop();
    await tester.pumpAndSettle();

    expect(
      _hubCalls,
      2,
      reason: 'home survives underneath a pushed route, so it must refetch',
    );
    final List<String> rails = tester
        .widgetList<CardRail>(find.byType(CardRail))
        .map((CardRail r) => r.title)
        .toList();
    expect(
      rails,
      <String>[Strings.recentlyAddedMovies],
      reason:
          'finishing a title takes it off the watchlist server-side, and the rail has to follow',
    );
  });
}
