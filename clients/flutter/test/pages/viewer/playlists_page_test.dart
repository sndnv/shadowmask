import 'dart:async';
import 'dart:convert';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/card_menu.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/components/setting_help.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/viewer/playlists_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _movie(String id, String title) => <String, dynamic>{
  'type': 'movie',
  'id': id,
  'title': title,
};

Map<String, dynamic> _episode() => <String, dynamic>{
  'type': 'episode',
  'id': 'e2',
  'season_id': 'se1',
  'series_title': 'The Show',
  'season_number': 1,
  'number': 2,
  'title': 'Second',
};

class _Server {
  _Server({
    bool queued = true,
    this.lists = true,
    this.failWrites = false,
    this.detailFailures = 0,
  }) : queue = queued
           ? <Map<String, dynamic>>[
               <String, dynamic>{'id': 'q1', 'card': _movie('m1', 'One')},
               <String, dynamic>{'id': 'q2', 'card': _episode()},
             ]
           : <Map<String, dynamic>>[];

  final List<Map<String, dynamic>> queue;
  final bool lists;
  final bool failWrites;
  int detailFailures;
  Completer<void>? moveGate;
  Completer<void>? readGate;
  final List<Completer<void>> queueReads = <Completer<void>>[];
  Completer<void>? listsReadGate;
  int queueReadFailures = 0;
  Completer<void>? dequeueGate;
  Completer<void>? enqueueGate;
  final List<http.Request> sent = <http.Request>[];

  int reads(String path) => sent
      .where((http.Request r) => r.method == 'GET' && r.url.path == path)
      .length;

  List<http.Request> writes() =>
      sent.where((http.Request r) => r.method != 'GET').toList();

  ApiClient api() => ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      sent.add(req);
      final String path = req.url.path;
      if (path == '/api/v1/users/self') {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'id': 'u1',
            'username': 'pat',
            'role': 'user',
          }),
          200,
        );
      }
      if (failWrites && req.method != 'GET') {
        return http.Response('{"message":"store is read only"}', 500);
      }
      if (req.method == 'PUT' && path.endsWith('/position')) {
        await moveGate?.future;
      }
      if (req.method == 'GET' && path == '/api/v1/users/u1/queue') {
        final String held = jsonEncode(queue);
        if (queueReads.isNotEmpty) {
          await queueReads.removeAt(0).future;
        }
        if (queueReadFailures > 0) {
          queueReadFailures--;
          return http.Response('{"message":"prefs.db is busy"}', 500);
        }
        return http.Response(held, 200);
      }
      if (req.method == 'GET' && path == '/api/v1/users/u1/playlists') {
        await listsReadGate?.future;
      }
      if (req.method == 'DELETE' && path.contains('/queue/items/')) {
        await dequeueGate?.future;
        final String entry = path.split('/').last;
        queue.removeWhere((Map<String, dynamic> e) => e['id'] == entry);
        return http.Response('', 204);
      }
      if (req.method == 'POST' && path == '/api/v1/users/u1/queue') {
        await enqueueGate?.future;
      }
      if (req.method == 'GET' && path == '/api/v1/users/u1/playlists/pl2') {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'id': 'pl2',
            'name': 'Empty',
            'created_at': '2026-11-06T10:00:00Z',
            'updated_at': '2026-11-06T10:00:00Z',
            'items': <dynamic>[],
          }),
          200,
        );
      }
      if (req.method == 'DELETE' && path == '/api/v1/users/u1/queue') {
        queue.clear();
        return http.Response('', 204);
      }
      if (req.method == 'GET' && path == '/api/v1/users/u1/playlists') {
        return http.Response(
          jsonEncode(<dynamic>[
            if (lists) ...<Map<String, dynamic>>[
              <String, dynamic>{
                'id': 'pl1',
                'name': 'Weekend',
                'item_count': 2,
                'created_at': '2026-11-06T10:00:00Z',
                'updated_at': '2026-11-06T10:00:00Z',
              },
              <String, dynamic>{
                'id': 'pl2',
                'name': 'Empty',
                'item_count': 0,
                'created_at': '2026-11-06T10:00:00Z',
                'updated_at': '2026-11-06T10:00:00Z',
              },
            ],
          ]),
          200,
        );
      }
      if (req.method == 'POST' && path == '/api/v1/users/u1/playlists') {
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
      if (req.method == 'GET' && path == '/api/v1/users/u1/playlists/pl1') {
        await readGate?.future;
        if (detailFailures > 0) {
          detailFailures--;
          return http.Response('{"message":"prefs.db is busy"}', 500);
        }
        return http.Response(
          jsonEncode(<String, dynamic>{
            'id': 'pl1',
            'name': 'Weekend',
            'created_at': '2026-11-06T10:00:00Z',
            'updated_at': '2026-11-06T10:00:00Z',
            'items': <dynamic>[
              <String, dynamic>{'position': 0, 'card': _movie('m1', 'One')},
              <String, dynamic>{'position': 2, 'card': _episode()},
              <String, dynamic>{'position': 3, 'card': _movie('m3', 'Three')},
            ],
          }),
          200,
        );
      }
      if (path.endsWith('/versions')) {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'items': <dynamic>[
              <String, dynamic>{
                'id': 'v1',
                'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
                'library_id': 'lib',
                'quality': 'hd',
                'container': 'mp4',
                'available': true,
              },
            ],
            'total': 1,
          }),
          200,
        );
      }
      if (req.method == 'POST' && path == '/api/v1/users/u1/queue') {
        return http.Response(jsonEncode(<String, dynamic>{'added': 3}), 200);
      }
      if (path == '/api/v1/users/u1/continue') {
        return http.Response('', 500);
      }
      return http.Response('', 204);
    }),
  );
}

Future<List<String>> _pump(
  WidgetTester tester,
  _Server server, {
  String? id,
  Size size = const Size(1400, 1000),
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final List<String> opened = <String>[];
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        navigatorObservers: <NavigatorObserver>[appRouteObserver],
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child!),
        home: PlaylistsPage(api: server.api(), id: id),
        onGenerateRoute: (RouteSettings settings) {
          opened.add(settings.name ?? '');
          return MaterialPageRoute<void>(
            settings: settings,
            builder: (BuildContext _) => const SizedBox(),
          );
        },
      ),
    ),
  );
  await tester.pumpAndSettle();
  return opened;
}

Future<void> _fromMenu(WidgetTester tester, String label) async {
  await tester.tap(find.byTooltip(Strings.moreActions));
  await tester.pumpAndSettle();
  await tester.tap(find.text(label).last);
  await tester.pumpAndSettle();
}

Map<String, dynamic> _body(http.Request r) =>
    jsonDecode(r.body) as Map<String, dynamic>;

Map<String, bool> _menuEnabled(WidgetTester tester) => <String, bool>{
  for (final PopupMenuItem<Object?> item
      in tester.widgetList<PopupMenuItem<Object?>>(
        find.byWidgetPredicate((Widget w) => w is PopupMenuItem),
      ))
    (item.child! as MenuRow).label: item.enabled,
};

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  group('the playlists section', () {
    testWidgets('the queue sits above the playlists and Play starts its '
        'first entry in queue mode', (WidgetTester tester) async {
      final List<String> opened = await _pump(tester, _Server());

      final String queue = Strings.countLabel(Strings.queueHeading, 2);
      expect(
        tester.getTopLeft(find.text(queue)).dy,
        lessThan(
          tester.getTopLeft(find.text(Strings.playlistsHeading).last).dy,
        ),
      );
      expect(find.text('One'), findsOneWidget);
      expect(find.text('The Show'), findsOneWidget);
      expect(find.text('Weekend'), findsOneWidget);
      expect(find.text(Strings.playlistItemCount(2)), findsOneWidget);

      await tester.tap(find.text(Strings.playQueue));
      await tester.pumpAndSettle();

      expect(opened, <String>[queuePlayRoute('q1')]);
    });

    testWidgets('Clear asks first, then empties the queue', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server);

      await tester.tap(find.text(Strings.clearQueue));
      await tester.pumpAndSettle();
      expect(find.text(Strings.confirmClearQueue), findsOneWidget);
      expect(server.writes(), isEmpty);

      await tester.tap(find.widgetWithText(FilledButton, Strings.clearQueue));
      await tester.pumpAndSettle();

      expect(server.writes().single.url.path, '/api/v1/users/u1/queue');
      expect(find.text(Strings.queueEmpty), findsOneWidget);
      expect(find.text(Strings.toastQueueCleared), findsOneWidget);
    });

    testWidgets('an entry\'s remove button takes just that entry', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server);

      await tester.tap(find.byTooltip(Strings.removeFromQueue).last);
      await tester.pumpAndSettle();

      final http.Request removed = server.writes().single;
      expect(removed.method, 'DELETE');
      expect(removed.url.path, '/api/v1/users/u1/queue/items/q2');
    });

    testWidgets('an empty queue has nothing to play or clear', (
      WidgetTester tester,
    ) async {
      await _pump(tester, _Server(queued: false));

      expect(find.text(Strings.queueEmpty), findsOneWidget);
      expect(find.text(Strings.queueHeading), findsOneWidget);
      for (final String label in <String>[
        Strings.playQueue,
        Strings.clearQueue,
      ]) {
        final ButtonStyleButton button = tester.widget<ButtonStyleButton>(
          find.ancestor(
            of: find.text(label),
            matching: find.bySubtype<ButtonStyleButton>(),
          ),
        );
        expect(button.onPressed, isNull, reason: label);
      }
    });

    testWidgets('New playlist asks for a name and makes it', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server);

      await tester.tap(find.text(Strings.newPlaylist));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), '   ');
      await tester.pump();
      final FilledButton blank = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, Strings.create),
      );
      expect(blank.onPressed, isNull);

      await tester.enterText(find.byType(TextField), '  Late ');
      await tester.pump();
      await tester.tap(find.widgetWithText(FilledButton, Strings.create));
      await tester.pumpAndSettle();

      final http.Request made = server.writes().single;
      expect(made.url.path, '/api/v1/users/u1/playlists');
      expect(_body(made), <String, dynamic>{'name': 'Late'});
    });

    testWidgets('a right-click on a playlist offers its actions, and adding '
        'it to the queue shows there at once', (WidgetTester tester) async {
      final _Server server = _Server();
      await _pump(tester, server);

      await tester.tap(find.text('Weekend'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();
      for (final String label in <String>[
        Strings.addToQueue,
        Strings.shuffleIntoQueue,
        Strings.shufflePlay,
        Strings.renamePlaylist,
        Strings.deletePlaylist,
      ]) {
        expect(find.text(label), findsOneWidget, reason: label);
      }

      await tester.tap(find.text(Strings.shuffleIntoQueue));
      await tester.pumpAndSettle();

      expect(_body(server.writes().single), <String, dynamic>{
        'playlist_id': 'pl1',
        'shuffle': true,
      });
      expect(server.reads('/api/v1/users/u1/queue'), 2);
    });

    testWidgets('Add to queue from the menu keeps the order', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server);

      await tester.tap(find.text('Weekend'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.addToQueue));
      await tester.pumpAndSettle();

      expect(_body(server.writes().single), <String, dynamic>{
        'playlist_id': 'pl1',
        'shuffle': false,
      });
    });

    testWidgets('a queue card\'s menu removal reads the queue again', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server);

      await tester.tap(find.text('One'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.removeFromQueue).last);
      await tester.pumpAndSettle();

      final http.Request removed = server
          .writes()
          .where((http.Request r) => r.method == 'DELETE')
          .single;
      expect(removed.url.path, '/api/v1/users/u1/queue/items/q1');
      expect(server.reads('/api/v1/users/u1/queue'), 2);
    });

    testWidgets('a shuffle that cannot read the playlist says so', (
      WidgetTester tester,
    ) async {
      final List<String> opened = await _pump(
        tester,
        _Server(detailFailures: 1),
      );

      await tester.tap(find.text('Weekend'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.shufflePlay));
      await tester.pumpAndSettle();

      expect(find.textContaining(Strings.errorAction), findsOneWidget);
      expect(opened, isEmpty);
    });

    testWidgets('an empty playlist\'s menu offers only Rename and Delete', (
      WidgetTester tester,
    ) async {
      await _pump(tester, _Server());

      await tester.tap(find.text('Empty'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();

      expect(_menuEnabled(tester), <String, bool>{
        Strings.addToQueue: false,
        Strings.shuffleIntoQueue: false,
        Strings.shufflePlay: false,
        Strings.renamePlaylist: true,
        Strings.deletePlaylist: true,
      });
    });

    testWidgets('a playlist\'s menu waits while another action runs', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server()..enqueueGate = Completer<void>();
      await _pump(tester, server);

      await tester.tap(find.text('Weekend'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.addToQueue));
      await tester.pump();
      await tester.tap(find.text('Weekend'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();

      expect(_menuEnabled(tester).values, everyElement(isFalse));

      await tester.tapAt(Offset.zero);
      server.enqueueGate!.complete();
      await tester.pumpAndSettle();
    });

    testWidgets('a removed queue entry leaves the rail in place while the '
        'queue is read again', (WidgetTester tester) async {
      final _Server server = _Server();
      await _pump(tester, server);
      final Completer<void> read = Completer<void>();
      server.queueReads.add(read);

      await tester.tap(find.byTooltip(Strings.removeFromQueue).first);
      await tester.pump();
      await tester.pump();

      expect(find.byType(CardRail), findsOneWidget);
      expect(find.byType(SkeletonRows), findsNothing);

      read.complete();
      await tester.pumpAndSettle();
      expect(find.text('One'), findsNothing);
    });

    testWidgets('the newest read of the queue wins when two overlap', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server);
      final Completer<void> first = Completer<void>();
      final Completer<void> second = Completer<void>();
      server.queueReads.addAll(<Completer<void>>[first, second]);

      await tester.tap(find.byTooltip(Strings.removeFromQueue).first);
      await tester.pump();
      await tester.pump();
      await tester.tap(find.byTooltip(Strings.removeFromQueue).last);
      await tester.pump();
      await tester.pump();
      second.complete();
      await tester.pump();
      await tester.pump();
      first.complete();
      await tester.pumpAndSettle();

      expect(find.text(Strings.queueEmpty), findsOneWidget);
      expect(find.text('The Show'), findsNothing);
    });

    testWidgets('a queue that cannot be read again after Clear shows the '
        'failure and a retry', (WidgetTester tester) async {
      final _Server server = _Server();
      await _pump(tester, server);
      server.queueReadFailures = 2;

      await tester.tap(find.text(Strings.clearQueue));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, Strings.clearQueue));
      await tester.pumpAndSettle();

      expect(find.text('One'), findsNothing);
      expect(find.text(Strings.retry), findsOneWidget);
    });

    testWidgets('a deleted playlist offers nothing until the list is read '
        'again', (WidgetTester tester) async {
      final _Server server = _Server();
      await _pump(tester, server);
      server.listsReadGate = Completer<void>();

      await tester.tap(find.text('Weekend'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.deletePlaylist));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, Strings.delete));
      await tester.pump();
      await tester.pump();
      await tester.tap(find.text('Weekend'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();

      expect(_menuEnabled(tester).values, everyElement(isFalse));

      await tester.tapAt(Offset.zero);
      server.listsReadGate!.complete();
      await tester.pumpAndSettle();
    });

    testWidgets('the row controls stay out of the screen reader\'s way', (
      WidgetTester tester,
    ) async {
      await _pump(tester, _Server(), id: 'pl1');

      final Iterable<GestureDetector> catchers = tester
          .widgetList<GestureDetector>(
            find.ancestor(
              of: find.byIcon(Icons.drag_handle).first,
              matching: find.byType(GestureDetector),
            ),
          )
          .where((GestureDetector g) => g.behavior == HitTestBehavior.opaque);
      expect(catchers, isNotEmpty);
      expect(
        catchers.every((GestureDetector g) => g.excludeFromSemantics),
        isTrue,
      );
    });

    testWidgets('a queue card\'s menu removal holds its own remove button', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server()..dequeueGate = Completer<void>();
      await _pump(tester, server);

      await tester.tap(find.text('One'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.removeFromQueue).last);
      await tester.pump();

      final CatalogCardTile tile = tester.widget<CatalogCardTile>(
        find.ancestor(
          of: find.text('One'),
          matching: find.byType(CatalogCardTile),
        ),
      );
      expect(tile.dismissBusy, isTrue);

      server.dequeueGate!.complete();
      await tester.pumpAndSettle();
      expect(
        server.sent.where((http.Request r) => r.method == 'DELETE'),
        hasLength(1),
      );
    });

    testWidgets('adding a queue card to a playlist reads the playlists again', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server);
      expect(server.reads('/api/v1/users/u1/playlists'), 1);

      await tester.tap(find.text('One'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.addToPlaylistEllipsis));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Weekend').last);
      await tester.pumpAndSettle();

      expect(server.reads('/api/v1/users/u1/playlists'), 3);
    });

    testWidgets('a long-press on a playlist renames it in place', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server);

      await tester.longPress(find.text('Weekend'));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.renamePlaylist));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), 'Sunday');
      await tester.tap(find.widgetWithText(FilledButton, Strings.save));
      await tester.pumpAndSettle();

      expect(_body(server.writes().single), <String, dynamic>{
        'name': 'Sunday',
      });
      expect(server.reads('/api/v1/users/u1/playlists'), 2);
    });

    testWidgets('a playlist deleted from its menu asks first', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server);

      await tester.tap(find.text('Weekend'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.deletePlaylist));
      await tester.pumpAndSettle();
      expect(
        find.text(Strings.confirmDeletePlaylist('Weekend')),
        findsOneWidget,
      );

      await tester.tap(find.widgetWithText(FilledButton, Strings.delete));
      await tester.pumpAndSettle();

      expect(server.writes().single.method, 'DELETE');
      expect(server.reads('/api/v1/users/u1/playlists'), 2);
    });

    testWidgets('Shuffle from the menu plays one of the playlist\'s titles', (
      WidgetTester tester,
    ) async {
      final List<String> opened = await _pump(tester, _Server());

      await tester.tap(find.text('Weekend'), buttons: kSecondaryButton);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.shufflePlay));
      await tester.pumpAndSettle();

      expect(Uri.parse(opened.single).queryParameters['version'], 'v1');
    });

    testWidgets('playlist rows are compact and keep their icons off the edge', (
      WidgetTester tester,
    ) async {
      await _pump(tester, _Server());

      final ListTile row = tester.widget<ListTile>(
        find.ancestor(
          of: find.text('Weekend'),
          matching: find.byType(ListTile),
        ),
      );
      expect(row.contentPadding, kSettingRowPadding);
      expect(row.dense, isTrue);
    });

    testWidgets('the queue rail spans the page, as home\'s rails do, so its '
        'cards come out the same size', (WidgetTester tester) async {
      await _pump(tester, _Server(), size: const Size(1100, 900));

      final double page = tester.getSize(find.byType(Breadcrumbs)).width;
      final double rail = tester.getSize(find.byType(CardRail)).width;
      final double card = tester
          .getSize(find.byType(CatalogCardTile).first)
          .width;

      expect(rail, page);
      expect(card, fittedCardWidth(page, CardAspect.poster, count: 2));
    });

    testWidgets('the queue stays on one row', (WidgetTester tester) async {
      await _pump(tester, _Server(), size: const Size(390, 844));

      expect(find.byType(CardRail), findsOneWidget);
      expect(
        tester.getTopLeft(find.text('One')).dy,
        tester.getTopLeft(find.text('The Show')).dy,
      );
    });

    testWidgets('no playlists yet says how to make one', (
      WidgetTester tester,
    ) async {
      await _pump(tester, _Server(lists: false));

      expect(find.text(Strings.playlistsEmpty), findsOneWidget);
    });

    testWidgets('a name can be confirmed with Enter', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server);

      await tester.tap(find.text(Strings.newPlaylist));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), 'Late');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();

      expect(_body(server.writes().single), <String, dynamic>{'name': 'Late'});
    });

    testWidgets('coming back to the section reads the queue and the '
        'playlists again', (WidgetTester tester) async {
      final _Server server = _Server();
      await _pump(tester, server);
      expect(server.reads('/api/v1/users/u1/queue'), 1);

      await tester.tap(find.text('Weekend'));
      await tester.pumpAndSettle();
      server.queue.removeAt(0);
      tester.state<NavigatorState>(find.byType(Navigator)).pop();
      await tester.pumpAndSettle();

      expect(server.reads('/api/v1/users/u1/queue'), 2);
      expect(server.reads('/api/v1/users/u1/playlists'), 2);
      expect(find.text('One'), findsNothing);
      expect(find.text('The Show'), findsOneWidget);
    });

    testWidgets('a playlist row opens the playlist', (
      WidgetTester tester,
    ) async {
      final List<String> opened = await _pump(tester, _Server());

      await tester.tap(find.text('Weekend'));
      await tester.pumpAndSettle();

      expect(opened, <String>[playlistRoute('pl1')]);
    });
  });

  group('a playlist', () {
    testWidgets('lists its titles in order, episodes under their series', (
      WidgetTester tester,
    ) async {
      await _pump(tester, _Server(), id: 'pl1');

      expect(find.text('Weekend'), findsWidgets);
      final double one = tester.getTopLeft(find.text('One')).dy;
      final double show = tester.getTopLeft(find.text('The Show')).dy;
      final double three = tester.getTopLeft(find.text('Three')).dy;
      expect(one < show && show < three, isTrue);
      expect(find.text('S01E02 · Second'), findsOneWidget);
    });

    testWidgets('an empty playlist says how to fill it and offers no queue', (
      WidgetTester tester,
    ) async {
      await _pump(tester, _Server(), id: 'pl2');

      expect(find.text(Strings.playlistEmpty), findsOneWidget);
      final ButtonStyleButton add = tester.widget<ButtonStyleButton>(
        find.ancestor(
          of: find.text(Strings.addToQueue),
          matching: find.bySubtype<ButtonStyleButton>(),
        ),
      );
      expect(add.onPressed, isNull);
    });

    testWidgets('a row opens its title', (WidgetTester tester) async {
      final List<String> opened = await _pump(tester, _Server(), id: 'pl1');

      await tester.tap(find.text('Three'));
      await tester.pumpAndSettle();

      expect(opened, <String>[movieRoute('m3')]);
    });

    testWidgets('a playlist that fails to load can be tried again', (
      WidgetTester tester,
    ) async {
      await _pump(tester, _Server(detailFailures: 1), id: 'pl1');
      expect(find.text('One'), findsNothing);

      await tester.tap(find.text(Strings.retry));
      await tester.pumpAndSettle();

      expect(find.text('One'), findsOneWidget);
    });

    testWidgets('a refused add to the queue says so and stays put', (
      WidgetTester tester,
    ) async {
      final List<String> opened = await _pump(
        tester,
        _Server(failWrites: true),
        id: 'pl1',
      );

      await tester.tap(find.text(Strings.addToQueue));
      await tester.pumpAndSettle();

      expect(find.textContaining(Strings.errorAction), findsOneWidget);
      expect(opened, isEmpty);
    });

    testWidgets('a refused drop says so and puts the rows back', (
      WidgetTester tester,
    ) async {
      await _pump(tester, _Server(failWrites: true), id: 'pl1');

      tester
          .widget<ReorderableListView>(find.byType(ReorderableListView))
          .onReorderItem!(0, 2);
      await tester.pumpAndSettle();

      expect(find.textContaining(Strings.errorAction), findsOneWidget);
      expect(
        tester.getTopLeft(find.text('One')).dy,
        lessThan(tester.getTopLeft(find.text('Three')).dy),
      );
    });

    testWidgets('a dropped row takes the stored position of the row it '
        'lands on', (WidgetTester tester) async {
      final _Server server = _Server();
      await _pump(tester, server, id: 'pl1');

      tester
          .widget<ReorderableListView>(find.byType(ReorderableListView))
          .onReorderItem!(0, 2);
      await tester.pumpAndSettle();

      final http.Request moved = server.writes().single;
      expect(moved.method, 'PUT');
      expect(
        moved.url.path,
        '/api/v1/users/u1/playlists/pl1/items/m1/position',
      );
      expect(_body(moved), <String, dynamic>{'position': 3});
    });

    testWidgets('a row moved up takes the stored position of the row it '
        'lands on', (WidgetTester tester) async {
      final _Server server = _Server();
      await _pump(tester, server, id: 'pl1');

      tester
          .widget<ReorderableListView>(find.byType(ReorderableListView))
          .onReorderItem!(2, 0);
      await tester.pumpAndSettle();

      final http.Request moved = server.writes().single;
      expect(
        moved.url.path,
        '/api/v1/users/u1/playlists/pl1/items/m3/position',
      );
      expect(_body(moved), <String, dynamic>{'position': 0});
    });

    testWidgets('no row can be dragged or removed until a move has been '
        'saved and read back', (WidgetTester tester) async {
      final _Server server = _Server()..moveGate = Completer<void>();
      await _pump(tester, server, id: 'pl1');
      server.readGate = Completer<void>();
      bool draggable() => tester
          .widgetList<ReorderableDragStartListener>(
            find.byType(ReorderableDragStartListener),
          )
          .every((ReorderableDragStartListener l) => l.enabled);
      bool removable() => tester
          .widgetList<IconButton>(find.widgetWithIcon(IconButton, Icons.close))
          .every((IconButton b) => b.onPressed != null);

      final ReorderableListView list = tester.widget<ReorderableListView>(
        find.byType(ReorderableListView),
      );
      list.onReorderItem!(0, 2);
      await tester.pump();
      expect((draggable(), removable()), (false, false));

      list.onReorderItem!(2, 0);
      server.moveGate!.complete();
      await tester.pump();
      await tester.pump();
      expect(
        draggable(),
        isFalse,
        reason: 'the rows still carry the old positions until the read-back',
      );

      server.readGate!.complete();
      await tester.pumpAndSettle();

      expect((draggable(), removable()), (true, true));
      expect(server.writes(), hasLength(1));
    });

    testWidgets('coming back to a playlist reads it again', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server, id: 'pl1');
      expect(server.reads('/api/v1/users/u1/playlists/pl1'), 1);

      await tester.tap(find.text('Three'));
      await tester.pumpAndSettle();
      tester.state<NavigatorState>(find.byType(Navigator)).pop();
      await tester.pumpAndSettle();

      expect(server.reads('/api/v1/users/u1/playlists/pl1'), 2);
    });

    testWidgets('a row\'s remove button takes that title off', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server, id: 'pl1');

      await tester.tap(find.byTooltip(Strings.removeFromPlaylist).first);
      await tester.pumpAndSettle();

      final http.Request removed = server.writes().single;
      expect(removed.method, 'DELETE');
      expect(removed.url.path, '/api/v1/users/u1/playlists/pl1/items/m1');
    });

    testWidgets('on a phone the two queue actions stand as icons and the '
        'rest wait in the menu', (WidgetTester tester) async {
      await _pump(tester, _Server(), id: 'pl1', size: const Size(390, 844));

      expect(find.byTooltip(Strings.addToQueue), findsOneWidget);
      expect(find.byTooltip(Strings.shuffleIntoQueue), findsOneWidget);
      expect(find.byTooltip(Strings.shufflePlay), findsNothing);

      await tester.tap(find.byTooltip(Strings.moreActions));
      await tester.pumpAndSettle();

      for (final String label in <String>[
        Strings.shufflePlay,
        Strings.renamePlaylist,
        Strings.deletePlaylist,
      ]) {
        expect(find.text(label), findsOneWidget, reason: label);
      }
    });

    for (final (String label, bool shuffle) in <(String, bool)>[
      (Strings.addToQueue, false),
      (Strings.shuffleIntoQueue, true),
    ]) {
      testWidgets('$label fills the queue and opens it', (
        WidgetTester tester,
      ) async {
        final _Server server = _Server();
        final List<String> opened = await _pump(tester, server, id: 'pl1');

        await tester.tap(find.text(label));
        await tester.pumpAndSettle();

        expect(_body(server.writes().single), <String, dynamic>{
          'playlist_id': 'pl1',
          'shuffle': shuffle,
        });
        expect(find.text(Strings.toastAddedToQueue(3)), findsOneWidget);
        expect(opened, <String>[playlistsRoute()]);
      });
    }

    testWidgets('a click on the drag handle opens nothing', (
      WidgetTester tester,
    ) async {
      final List<String> opened = await _pump(tester, _Server(), id: 'pl1');

      await tester.tap(find.byIcon(Icons.drag_handle).first);
      await tester.pumpAndSettle();

      expect(opened, isEmpty);
    });

    testWidgets('a click on a remove button that is waiting opens nothing', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server()..moveGate = Completer<void>();
      final List<String> opened = await _pump(tester, server, id: 'pl1');

      tester
          .widget<ReorderableListView>(find.byType(ReorderableListView))
          .onReorderItem!(0, 2);
      await tester.pump();
      await tester.tap(find.byTooltip(Strings.removeFromPlaylist).first);
      await tester.pump();

      expect(opened, isEmpty);

      server.moveGate!.complete();
      await tester.pumpAndSettle();
    });

    testWidgets('the whole row opens its title, art included', (
      WidgetTester tester,
    ) async {
      final List<String> opened = await _pump(tester, _Server(), id: 'pl1');

      await tester.tap(find.byType(CardArt).last);
      await tester.pumpAndSettle();

      expect(opened, <String>[movieRoute('m3')]);
    });

    testWidgets('Shuffle plays one title outside the queue', (
      WidgetTester tester,
    ) async {
      final List<String> opened = await _pump(tester, _Server(), id: 'pl1');

      await _fromMenu(tester, Strings.shufflePlay);

      final Uri played = Uri.parse(opened.single);
      expect(played.path, '/watch');
      expect(played.queryParameters['version'], 'v1');
      expect(played.queryParameters.containsKey('queue'), isFalse);
    });

    testWidgets('Rename starts from the current name', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      await _pump(tester, server, id: 'pl1');

      await _fromMenu(tester, Strings.renamePlaylist);
      expect(
        tester.widget<TextField>(find.byType(TextField)).controller?.text,
        'Weekend',
      );
      await tester.enterText(find.byType(TextField), 'Sunday');
      await tester.tap(find.widgetWithText(FilledButton, Strings.save));
      await tester.pumpAndSettle();

      final http.Request renamed = server.writes().single;
      expect(renamed.method, 'PUT');
      expect(renamed.url.path, '/api/v1/users/u1/playlists/pl1');
      expect(_body(renamed), <String, dynamic>{'name': 'Sunday'});
    });

    testWidgets('Delete asks first, then returns to the playlists', (
      WidgetTester tester,
    ) async {
      final _Server server = _Server();
      final List<String> opened = await _pump(tester, server, id: 'pl1');

      await _fromMenu(tester, Strings.deletePlaylist);
      expect(
        find.text(Strings.confirmDeletePlaylist('Weekend')),
        findsOneWidget,
      );
      expect(server.writes(), isEmpty);

      await tester.tap(find.widgetWithText(FilledButton, Strings.delete));
      await tester.pumpAndSettle();

      final http.Request deleted = server.writes().single;
      expect(deleted.method, 'DELETE');
      expect(deleted.url.path, '/api/v1/users/u1/playlists/pl1');
      expect(opened, <String>[playlistsRoute()]);
    });
  });
}
