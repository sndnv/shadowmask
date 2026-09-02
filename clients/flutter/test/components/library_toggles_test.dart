import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/library_toggles.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  Future<void> pumpToggles(
    WidgetTester tester,
    List<http.Request> seen, {
    bool showWatchlistFavorite = true,
    TitleRef ref = const TitleRef(type: TitleKind.movie, id: 'm1'),
    int episodeCount = 0,
    bool initialWatched = false,
    bool initialWatchlisted = false,
    double? width,
  }) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        seen.add(req);
        return http.Response('', 204);
      }),
    );
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: SizedBox(
              width: width,
              child: LibraryToggles(
                catalog: CatalogApi(api),
                userId: 'u1',
                ref: ref,
                title: 'Big Buck Bunny',
                showWatchlistFavorite: showWatchlistFavorite,
                episodeCount: episodeCount,
                initialWatched: initialWatched,
                initialWatchlisted: initialWatchlisted,
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('tapping the watched toggle sends the watched request', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = <http.Request>[];
    await pumpToggles(tester, seen);

    await tester.tap(find.byIcon(Icons.check));
    await tester.pump();
    await tester.pump();

    expect(
      seen.any(
        (http.Request r) =>
            r.method == 'PUT' && r.url.path == '/api/v1/users/u1/watched/m1',
      ),
      isTrue,
    );

    await tester.pump(const Duration(milliseconds: 3300));
  });

  testWidgets('marking watched also takes the title off the watchlist', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = <http.Request>[];
    await pumpToggles(tester, seen, initialWatchlisted: true);

    expect(
      find.byIcon(Icons.bookmark),
      findsOneWidget,
      reason: 'the title starts out saved',
    );

    await tester.tap(find.byIcon(Icons.check));
    await tester.pump();
    await tester.pump();

    expect(
      find.byIcon(Icons.bookmark_border),
      findsOneWidget,
      reason:
          'the server drops a watched title from the watchlist, so the bookmark has to follow',
    );
    expect(
      seen.where((http.Request r) => r.method == 'DELETE'),
      isEmpty,
      reason: 'the server already removed it, a second delete would be a lie',
    );

    await tester.pump(const Duration(milliseconds: 3300));
  });

  testWidgets('marking unwatched leaves the watchlist alone', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = <http.Request>[];
    await pumpToggles(
      tester,
      seen,
      initialWatched: true,
      initialWatchlisted: true,
    );

    await tester.tap(find.byIcon(Icons.check));
    await tester.pump();
    await tester.pump();

    expect(
      find.byIcon(Icons.bookmark),
      findsOneWidget,
      reason: 'only marking watched removes, unwatching puts nothing back',
    );

    await tester.pump(const Duration(milliseconds: 3300));
  });

  testWidgets('a reload from the page overrides the captured state', (
    WidgetTester tester,
  ) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient(
        (http.Request req) async => http.Response('', 204),
      ),
    );
    late StateSetter rebuild;
    bool watchlisted = true;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: StatefulBuilder(
              builder: (BuildContext context, StateSetter setState) {
                rebuild = setState;
                return LibraryToggles(
                  catalog: CatalogApi(api),
                  userId: 'u1',
                  ref: const TitleRef(type: TitleKind.movie, id: 'm1'),
                  title: 'Big Buck Bunny',
                  initialWatchlisted: watchlisted,
                );
              },
            ),
          ),
        ),
      ),
    );

    expect(find.byIcon(Icons.bookmark), findsOneWidget);

    rebuild(() {
      watchlisted = false;
    });
    await tester.pump();

    expect(
      find.byIcon(Icons.bookmark_border),
      findsOneWidget,
      reason: 'without didUpdateWidget the first value would stick forever',
    );
  });

  testWidgets('series shows only the watched toggle', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = <http.Request>[];
    await pumpToggles(
      tester,
      seen,
      showWatchlistFavorite: false,
      ref: const TitleRef(type: TitleKind.series, id: 's1'),
    );

    expect(find.byIcon(Icons.check), findsOneWidget);
    expect(find.byIcon(Icons.bookmark_border), findsNothing);
    expect(find.byIcon(Icons.favorite_border), findsNothing);
  });

  testWidgets('a fan-out over several episodes asks before writing', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = <http.Request>[];
    await pumpToggles(
      tester,
      seen,
      showWatchlistFavorite: false,
      ref: const TitleRef(type: TitleKind.season, id: 'se1'),
      episodeCount: 12,
    );

    await tester.tap(find.byIcon(Icons.check));
    await tester.pumpAndSettle();

    expect(
      find.text('Are you sure you want to mark 12 episodes as watched?'),
      findsOneWidget,
    );
    expect(seen, isEmpty);

    await tester.tap(find.widgetWithText(FilledButton, 'Mark watched'));
    await tester.pumpAndSettle();

    expect(
      seen.any(
        (http.Request r) =>
            r.method == 'PUT' && r.url.path == '/api/v1/users/u1/watched/se1',
      ),
      isTrue,
    );

    await tester.pump(const Duration(milliseconds: 3300));
  });

  testWidgets('declining the fan-out writes nothing', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = <http.Request>[];
    await pumpToggles(
      tester,
      seen,
      showWatchlistFavorite: false,
      ref: const TitleRef(type: TitleKind.season, id: 'se1'),
      episodeCount: 12,
    );

    await tester.tap(find.byIcon(Icons.check));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.close));
    await tester.pumpAndSettle();

    expect(seen, isEmpty);
  });

  testWidgets('a single episode does not ask', (WidgetTester tester) async {
    final List<http.Request> seen = <http.Request>[];
    await pumpToggles(
      tester,
      seen,
      showWatchlistFavorite: false,
      ref: const TitleRef(type: TitleKind.season, id: 'se1'),
      episodeCount: 1,
    );

    await tester.tap(find.byIcon(Icons.check));
    await tester.pumpAndSettle();

    expect(find.byType(FilledButton), findsNothing);
    expect(
      seen.any(
        (http.Request r) =>
            r.method == 'PUT' && r.url.path == '/api/v1/users/u1/watched/se1',
      ),
      isTrue,
    );

    await tester.pump(const Duration(milliseconds: 3300));
  });

  testWidgets('a wide row keeps the labels', (WidgetTester tester) async {
    await pumpToggles(tester, <http.Request>[], width: 700);

    expect(find.text(Strings.watchedLabel), findsOneWidget);
    expect(find.text(Strings.watchlistLabel), findsOneWidget);
    expect(find.text(Strings.favoriteLabel), findsOneWidget);
  });

  testWidgets('a phone row drops to icons that still announce themselves', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await pumpToggles(tester, <http.Request>[], width: 304);

    expect(find.text(Strings.watchedLabel), findsNothing);
    expect(find.byIcon(Icons.check), findsOneWidget);

    // An icon-only control is unguessable without a tooltip and a label.
    expect(find.byTooltip(Strings.markWatched), findsOneWidget);
    expect(
      tester.getSemantics(find.byIcon(Icons.bookmark_border)).label,
      contains(Strings.watchlistLabel),
    );
    semantics.dispose();
  });
}
