import 'dart:convert';

import 'package:material_ui/material_ui.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playlist_api.dart';
import 'package:shadowmask/components/card_menu.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/model/user_library/item_state.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shared_preferences/shared_preferences.dart';

const TitleRef _ref = TitleRef(type: TitleKind.movie, id: 'm1');

ItemState _state(TitleKind kind) => ItemState(
  title: TitleRef(type: kind, id: 'x'),
);

List<String> _labels(List<CardMenuAction> actions) => <String>[
  for (final CardMenuAction a in actions) a.label,
];

CardMenuAction _action(List<CardMenuAction> actions, CardAction of) =>
    actions.firstWhere((CardMenuAction a) => a.action == of);

double _menuWidth(WidgetTester tester, String anyLabel) => tester
    .getSize(
      find
          .ancestor(of: find.text(anyLabel), matching: find.byType(Material))
          .last,
    )
    .width;

Color? _labelColor(WidgetTester tester, String label) =>
    tester.widget<Text>(find.text(label)).style?.color;

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('an untouched title offers only the adding side of each toggle', () {
    final List<String> labels = _labels(
      cardMenuActions(const ItemState(title: _ref), dismissible: false),
    );

    expect(labels, <String>[
      Strings.markWatched,
      Strings.addWatchlist,
      Strings.addFavorite,
    ]);
  });

  test('each toggle flips to its undo once the state is already set', () {
    final List<String> labels = _labels(
      cardMenuActions(
        const ItemState(
          title: _ref,
          watched: true,
          watchlisted: true,
          favorite: true,
        ),
        dismissible: false,
      ),
    );

    expect(labels, <String>[
      Strings.markUnwatched,
      Strings.removeWatchlist,
      Strings.removeFavorite,
    ]);
  });

  test('only a dismissible card is offered the Continue watching removal', () {
    expect(
      _labels(cardMenuActions(const ItemState(title: _ref), dismissible: true)),
      contains(Strings.dismissResume),
    );
    expect(
      _labels(
        cardMenuActions(const ItemState(title: _ref), dismissible: false),
      ),
      isNot(contains(Strings.dismissResume)),
    );
  });

  test('a season or series is offered watched alone', () {
    for (final TitleKind kind in <TitleKind>[
      TitleKind.season,
      TitleKind.series,
    ]) {
      expect(
        _labels(cardMenuActions(_state(kind), dismissible: false)),
        <String>[Strings.markWatched],
        reason: 'watchlist and favorites are movie and episode only',
      );
    }
  });

  test('a person or collection is offered nothing, so it opens no menu', () {
    for (final TitleKind kind in <TitleKind>[
      TitleKind.person,
      TitleKind.collection,
    ]) {
      expect(cardMenuActions(_state(kind), dismissible: false), isEmpty);
      expect(hasCardMenu(kind, dismissible: false), isFalse);
    }
  });

  test('every kind the server takes a call for still opens a menu', () {
    for (final TitleKind kind in <TitleKind>[
      TitleKind.movie,
      TitleKind.episode,
      TitleKind.season,
      TitleKind.series,
    ]) {
      expect(hasCardMenu(kind, dismissible: false), isTrue);
    }
  });

  test('the dismissal is separated from the toggles by a divider', () {
    final List<PopupMenuEntry<CardAction>> entries = cardMenuEntries(
      const ItemState(title: _ref),
      dismissible: true,
    );

    expect(entries.whereType<PopupMenuDivider>(), hasLength(1));
  });

  test('the separator is inset to 80% of the menu, not the full width', () {
    final PopupMenuDivider rule = cardMenuEntries(
      const ItemState(title: _ref),
      dismissible: true,
    ).whereType<PopupMenuDivider>().single;

    expect(rule.indent, kCardMenuWidth * 0.1);
    expect(rule.endIndent, kCardMenuWidth * 0.1);
    expect(
      kCardMenuWidth - rule.indent! - rule.endIndent!,
      kCardMenuWidth * 0.8,
    );
  });

  test('every action carries the glyph the Roku menu gives it', () {
    final List<CardMenuAction> fresh = cardMenuActions(
      const ItemState(title: _ref),
      dismissible: true,
    );

    expect(_action(fresh, CardAction.watched).icon, Icons.check);
    expect(_action(fresh, CardAction.watchlist).icon, Icons.bookmark_border);
    expect(_action(fresh, CardAction.favorite).icon, Icons.favorite_border);
    expect(_action(fresh, CardAction.dismiss).icon, Icons.close);
  });

  test('taking something away is destructive, adding it is not', () {
    final List<CardMenuAction> fresh = cardMenuActions(
      const ItemState(title: _ref),
      dismissible: true,
    );
    final List<CardMenuAction> set = cardMenuActions(
      const ItemState(
        title: _ref,
        watched: true,
        watchlisted: true,
        favorite: true,
      ),
      dismissible: false,
    );

    expect(_action(fresh, CardAction.watchlist).danger, isFalse);
    expect(_action(fresh, CardAction.favorite).danger, isFalse);
    expect(_action(fresh, CardAction.dismiss).danger, isTrue);
    expect(_action(set, CardAction.watchlist).danger, isTrue);
    expect(_action(set, CardAction.favorite).danger, isTrue);
    expect(
      _action(set, CardAction.watched).danger,
      isFalse,
      reason: 'watched toggles both ways, it takes nothing away',
    );
  });

  test('a removal drops the toggle glyph for the closing one', () {
    final List<CardMenuAction> set = cardMenuActions(
      const ItemState(title: _ref, watchlisted: true, favorite: true),
      dismissible: false,
    );

    expect(_action(set, CardAction.watchlist).icon, Icons.close);
    expect(_action(set, CardAction.favorite).icon, Icons.close);
  });

  test('only a season or series is worth stopping to ask about', () {
    for (final bool watched in <bool>[true, false]) {
      expect(
        watchedConfirmBody(TitleKind.movie, watched: watched),
        isNull,
        reason: 'one title is one click to put back',
      );
      expect(watchedConfirmBody(TitleKind.episode, watched: watched), isNull);
      expect(watchedConfirmBody(TitleKind.season, watched: watched), isNotNull);
      expect(watchedConfirmBody(TitleKind.series, watched: watched), isNotNull);
    }
  });

  test('the question names the kind and the direction it is going', () {
    expect(
      watchedConfirmBody(TitleKind.series, watched: true),
      Strings.confirmWatchedSeries,
    );
    expect(
      watchedConfirmBody(TitleKind.series, watched: false),
      Strings.confirmUnwatchedSeries,
    );
    expect(
      watchedConfirmBody(TitleKind.season, watched: true),
      Strings.confirmWatchedSeason,
    );
    expect(
      watchedConfirmBody(TitleKind.season, watched: false),
      Strings.confirmUnwatchedSeason,
    );
  });

  test('the menu opens at the pointer, measured from both edges', () {
    final RelativeRect at = menuPositionAt(
      const Offset(300, 200),
      const Size(1000, 800),
    );

    expect(at.left, 300);
    expect(at.top, 200);
    expect(at.right, 700);
    expect(at.bottom, 600);
  });

  testWidgets('a card the page already tagged opens without a round trip', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    final CatalogCard card = _card(watchlisted: true, stateKnown: true);

    await _open(tester, card, seen);

    expect(
      seen,
      isEmpty,
      reason: 'the state came back with the page, so asking again only stalls',
    );
    expect(find.text(Strings.removeWatchlist), findsOneWidget);
  });

  testWidgets('a card with no state of its own is still fetched', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];

    await _open(tester, _card(), seen);

    expect(seen, <String>['POST /api/v1/users/u1/state/batch']);
    expect(find.text(Strings.addWatchlist), findsOneWidget);
  });

  testWidgets('the menu is fully drawn on the frame after the click', (
    WidgetTester tester,
  ) async {
    await _open(tester, _card(stateKnown: true), <String>[], settle: false);
    await tester.pump();

    final Finder sheet = find
        .ancestor(
          of: find.text(Strings.markWatched),
          matching: find.byType(Material),
        )
        .first;
    final Size onOpening = tester.getSize(sheet);
    await tester.pumpAndSettle();

    expect(
      tester.getSize(sheet),
      onOpening,
      reason: 'a context menu should pop into existence, not grow into it',
    );
  });

  testWidgets('a person card opens nothing and asks the server nothing', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];

    await _open(tester, _card(kind: TitleKind.person), seen);

    expect(seen, isEmpty);
    expect(find.text(Strings.markWatched), findsNothing);
  });

  testWidgets('an untagged series skips the fetch its kind cannot make', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];

    await _open(tester, _card(kind: TitleKind.series), seen);

    expect(
      seen,
      isEmpty,
      reason: 'the batch endpoint takes a movie or an episode only',
    );
    expect(find.text(Strings.markWatched), findsOneWidget);
    expect(find.text(Strings.addWatchlist), findsNothing);
  });

  testWidgets('the menu holds one width whatever it is asked to hold', (
    WidgetTester tester,
  ) async {
    await _open(tester, _card(stateKnown: true), <String>[]);
    final double onMovie = _menuWidth(tester, Strings.markWatched);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    await _open(tester, _card(kind: TitleKind.series), <String>[]);

    expect(
      _menuWidth(tester, Strings.markWatched),
      onMovie,
      reason: 'one short item should not shrink the menu around itself',
    );
    expect(onMovie, kCardMenuWidth);
  });

  testWidgets('a label is given every pixel the fixed width leaves it', (
    WidgetTester tester,
  ) async {
    await _open(tester, _card(stateKnown: true), <String>[], onDismiss: () {});

    expect(
      tester.getSize(find.text(Strings.dismissResume)).width,
      kCardMenuWidth - Space.s3 * 2 - 24 - Space.s2,
      reason: 'the item padding, the icon slot and its gap, and nothing else',
    );
  });

  testWidgets('a destructive item is red, an ordinary one is not', (
    WidgetTester tester,
  ) async {
    await _open(tester, _card(watchlisted: true, stateKnown: true), <String>[]);

    const Tokens t = Tokens.dark;
    expect(_labelColor(tester, Strings.removeWatchlist), t.danger);
    expect(_labelColor(tester, Strings.markWatched), t.text);
  });

  testWidgets('choosing the Continue watching removal runs it', (
    WidgetTester tester,
  ) async {
    int dismissed = 0;

    await _open(
      tester,
      _card(stateKnown: true),
      <String>[],
      onDismiss: () => dismissed += 1,
    );
    await tester.tap(find.text(Strings.dismissResume));
    await tester.pumpAndSettle();

    expect(dismissed, 1);
  });

  testWidgets('marking a series watched asks before it fans out', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];

    await _open(tester, _card(kind: TitleKind.series), seen);
    await tester.tap(find.text(Strings.markWatched));
    await tester.pumpAndSettle();

    expect(find.text(Strings.confirmWatchedSeries), findsOneWidget);
    expect(seen, isEmpty, reason: 'nothing is sent until the question is met');

    await tester.tap(find.text(Strings.cancel));
    await tester.pumpAndSettle();

    expect(
      seen,
      isEmpty,
      reason: 'declining has to leave every episode where it was',
    );
  });

  testWidgets('accepting the question is what sends the series call', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];

    await _open(tester, _card(kind: TitleKind.series), seen);
    await tester.tap(find.text(Strings.markWatched));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.markWatched).last);
    await tester.pumpAndSettle();

    expect(seen, <String>['PUT /api/v1/users/u1/watched/m1']);
  });

  testWidgets('a movie is still marked watched without being asked', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];

    await _open(tester, _card(stateKnown: true), seen);
    await tester.tap(find.text(Strings.markWatched));
    await tester.pumpAndSettle();

    expect(seen, <String>['PUT /api/v1/users/u1/watched/m1']);
  });

  testWidgets('a fetched state is kept, so the second open is instant', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    final CatalogCard card = _card();

    await _open(tester, card, seen);
    expect(card.stateKnown, isTrue);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    await _open(tester, card, seen);

    expect(seen, hasLength(1));
  });

  test('every title a playlist can hold is offered Add to playlist', () {
    for (final TitleKind kind in TitleKind.values) {
      final List<String> labels = _labels(
        cardMenuActions(_state(kind), dismissible: false, playlists: true),
      );
      expect(
        labels.contains(Strings.addToPlaylistEllipsis),
        kind.isWatchTarget,
        reason: kind.wire,
      );
    }
    expect(
      _labels(cardMenuActions(_state(TitleKind.movie), dismissible: false)),
      isNot(contains(Strings.addToPlaylistEllipsis)),
      reason: 'a page with no playlist service offers nothing to add to',
    );
  });

  testWidgets('a picked playlist takes the title and says so', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _open(
      tester,
      _card(stateKnown: true, kind: TitleKind.season),
      seen,
      playlists: _playlists(seen, added: 4),
    );

    await tester.tap(find.text(Strings.addToPlaylistEllipsis));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Weekend'));
    await tester.pumpAndSettle();

    expect(
      seen.last,
      'POST /api/v1/users/u1/playlists/pl1/items {"type":"season","id":"m1"}',
    );
    expect(
      find.text(Strings.toastAddedToPlaylist(4, 'Weekend')),
      findsOneWidget,
    );
  });

  testWidgets('a title already in the playlist is reported, not added twice', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _open(
      tester,
      _card(stateKnown: true),
      seen,
      playlists: _playlists(seen, added: 0),
    );

    await tester.tap(find.text(Strings.addToPlaylistEllipsis));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Weekend'));
    await tester.pumpAndSettle();

    expect(
      find.text(Strings.toastAlreadyInPlaylist('Weekend')),
      findsOneWidget,
    );
  });

  testWidgets('a new playlist is made and takes the title', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _open(
      tester,
      _card(stateKnown: true),
      seen,
      playlists: _playlists(seen, added: 1),
    );

    await tester.tap(find.text(Strings.addToPlaylistEllipsis));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.newPlaylistEllipsis));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'Late');
    await tester.pump();
    await tester.tap(find.widgetWithText(FilledButton, Strings.create));
    await tester.pumpAndSettle();

    expect(seen.skip(1), <String>[
      'POST /api/v1/users/u1/playlists {"name":"Late"}',
      'POST /api/v1/users/u1/playlists/pl9/items {"type":"movie","id":"m1"}',
    ]);
    expect(find.text(Strings.toastAddedToPlaylist(1, 'Late')), findsOneWidget);
  });

  test('only a queue card is offered its removal from the queue', () {
    expect(
      _labels(
        cardMenuActions(
          _state(TitleKind.movie),
          dismissible: false,
          playlists: true,
          queued: true,
        ),
      ),
      contains(Strings.removeFromQueue),
    );
    expect(
      _labels(
        cardMenuActions(
          _state(TitleKind.movie),
          dismissible: false,
          playlists: true,
        ),
      ),
      isNot(contains(Strings.removeFromQueue)),
    );
  });

  testWidgets('a queue card drops its entry and leaves the watched state', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    final List<CardAction> changed = <CardAction>[];
    final CatalogCard card =
        CatalogCard.fromJson(<String, dynamic>{
            'type': 'movie',
            'id': 'm1',
            'title': 'Alpha',
          }).queued('7')
          ..stateKnown = true
          ..watched = true;
    await _open(
      tester,
      card,
      seen,
      playlists: _playlists(seen, added: 0),
      onChanged: changed.add,
    );

    expect(find.text(Strings.markUnwatched), findsOneWidget);
    await tester.tap(find.text(Strings.removeFromQueue));
    await tester.pumpAndSettle();

    expect(seen, <String>['DELETE /api/v1/users/u1/queue/items/7']);
    expect(changed, <CardAction>[CardAction.dequeue]);
    expect(find.text(Strings.toastRemovedFromQueue('Alpha')), findsOneWidget);
  });

  testWidgets('a refused removal from the queue says so', (
    WidgetTester tester,
  ) async {
    final CatalogCard card = CatalogCard.fromJson(<String, dynamic>{
      'type': 'movie',
      'id': 'm1',
      'title': 'Alpha',
    }).queued('7')..stateKnown = true;
    await _open(
      tester,
      card,
      <String>[],
      playlists: _playlists(<String>[], added: 0, writeFails: true),
    );

    await tester.tap(find.text(Strings.removeFromQueue));
    await tester.pumpAndSettle();

    expect(find.textContaining(Strings.errorRemove), findsOneWidget);
  });

  testWidgets('an add reports the change, and so does a playlist made in the '
      'picker even when the add then fails', (WidgetTester tester) async {
    for (final bool addFails in <bool>[false, true]) {
      final List<CardAction> changed = <CardAction>[];
      await _open(
        tester,
        _card(stateKnown: true),
        <String>[],
        playlists: _playlists(<String>[], added: 1, addFails: addFails),
        onChanged: changed.add,
      );

      await tester.tap(find.text(Strings.addToPlaylistEllipsis));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.newPlaylistEllipsis));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), 'Late');
      await tester.pump();
      await tester.tap(find.widgetWithText(FilledButton, Strings.create));
      await tester.pumpAndSettle();

      expect(changed, <CardAction>[CardAction.playlist], reason: '$addFails');
    }
  });

  testWidgets('a picker that cannot read the playlists says so', (
    WidgetTester tester,
  ) async {
    await _open(
      tester,
      _card(stateKnown: true),
      <String>[],
      playlists: _playlists(<String>[], added: 0, readFails: true),
    );

    await tester.tap(find.text(Strings.addToPlaylistEllipsis));
    await tester.pumpAndSettle();

    expect(find.text(Strings.couldNotLoadPlaylists), findsOneWidget);
  });

  testWidgets('a refused add or a refused new playlist says so', (
    WidgetTester tester,
  ) async {
    await _open(
      tester,
      _card(stateKnown: true),
      <String>[],
      playlists: _playlists(<String>[], added: 0, writeFails: true),
    );

    await tester.tap(find.text(Strings.addToPlaylistEllipsis));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.newPlaylistEllipsis));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'Late');
    await tester.pump();
    await tester.tap(find.widgetWithText(FilledButton, Strings.create));
    await tester.pumpAndSettle();

    expect(find.textContaining(Strings.errorAction), findsOneWidget);
    expect(find.text('Weekend'), findsOneWidget, reason: 'the picker stays');

    await tester.tap(find.text('Weekend'));
    await tester.pumpAndSettle();

    expect(find.textContaining(Strings.errorAction), findsWidgets);
    expect(find.text('Weekend'), findsNothing);
  });
}

PlaylistApi _playlists(
  List<String> seen, {
  required int added,
  bool readFails = false,
  bool writeFails = false,
  bool addFails = false,
}) => PlaylistApi(
  ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      seen.add(
        '${req.method} ${req.url.path}${req.body.isEmpty ? '' : ' ${req.body}'}',
      );
      if (req.method == 'GET' && readFails ||
          req.method != 'GET' && writeFails ||
          addFails && req.url.path.endsWith('/items')) {
        return http.Response('{"message":"prefs.db is busy"}', 500);
      }
      if (req.method == 'GET') {
        return http.Response(
          jsonEncode(<dynamic>[
            <String, dynamic>{
              'id': 'pl1',
              'name': 'Weekend',
              'item_count': 2,
              'created_at': '2026-11-06T10:00:00Z',
              'updated_at': '2026-11-06T10:00:00Z',
            },
          ]),
          200,
        );
      }
      if (req.url.path.endsWith('/playlists')) {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'id': 'pl9',
            'name': 'Late',
            'created_at': '2026-11-06T10:00:00Z',
            'updated_at': '2026-11-06T10:00:00Z',
          }),
          201,
        );
      }
      return http.Response(jsonEncode(<String, dynamic>{'added': added}), 200);
    }),
  ),
);

CatalogCard _card({
  bool watchlisted = false,
  bool stateKnown = false,
  TitleKind kind = TitleKind.movie,
}) => CatalogCard(
  ref: TitleRef(type: kind, id: 'm1'),
  route: '/title?type=${kind.wire}&id=m1',
  title: 'Alpha',
  watchlisted: watchlisted,
  stateKnown: stateKnown,
);

Future<void> _open(
  WidgetTester tester,
  CatalogCard card,
  List<String> seen, {
  bool settle = true,
  VoidCallback? onDismiss,
  PlaylistApi? playlists,
  void Function(CardAction action)? onChanged,
}) async {
  final CatalogApi catalog = CatalogApi(
    ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        seen.add('${req.method} ${req.url.path}');
        return http.Response('[]', 200);
      }),
    ),
  );
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      builder: (BuildContext context, Widget? child) =>
          ToastHost(child: child!),
      home: Scaffold(
        body: Builder(
          builder: (BuildContext context) => TextButton(
            onPressed: () => showCardMenu(
              context,
              catalog: catalog,
              userId: 'u1',
              card: card,
              at: Offset.zero,
              onDismiss: onDismiss,
              playlists: playlists,
              onChanged: onChanged,
            ),
            child: const Text('open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  if (settle) {
    await tester.pumpAndSettle();
  }
}
