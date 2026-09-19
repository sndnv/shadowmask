import 'dart:convert';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/capability_scope.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/components/segmented_tabs.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/pages/account/account_page.dart';
import 'package:shadowmask/pages/account/playback_support_block.dart';
import 'package:shadowmask/pages/account/profile_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

http.Response _route(http.Request req, String role) {
  final String path = req.url.path;
  if (path == '/api/v1/users/self') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'id': 'u1',
        'username': 'pat',
        'role': role,
      }),
      200,
    );
  }
  if (path == '/api/v1/users/u1') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'id': 'u1',
        'username': 'pat',
        'role': role,
        'preferred_audio': <String>[],
        'preferred_subtitle': <String>[],
        'created_at': '2026-01-01T00:00:00Z',
        'updated_at': '2026-01-01T00:00:00Z',
      }),
      200,
    );
  }
  if (path == '/api/v1/users/u1/history') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'items': <dynamic>[],
        'total': 0,
        'offset': 0,
        'limit': 0,
      }),
      200,
    );
  }
  if (path == '/api/v1/users/u1/devices') {
    return http.Response(
      jsonEncode(<dynamic>[
        <String, dynamic>{
          'id': 'd1',
          'user_id': 'u1',
          'name': 'Lounge tablet',
          'platform': 'android',
          'created_at': '2026-08-17T09:00:00Z',
          'last_seen': '2026-08-17T09:24:11Z',
        },
      ]),
      200,
    );
  }
  if (path == '/api/v1/users/u1/watchlist') {
    return http.Response(
      jsonEncode(<dynamic>[
        for (final String id in _watchlisted)
          <String, dynamic>{
            'user_id': 'u1',
            'title': <String, dynamic>{'type': 'movie', 'id': id},
            'added_at': '2026-01-01T00:00:00Z',
          },
      ]),
      200,
    );
  }
  if (path == '/api/v1/users/u1/favorites') {
    return http.Response(
      jsonEncode(<dynamic>[
        for (final String id in _favorited)
          <String, dynamic>{
            'user_id': 'u1',
            'title': <String, dynamic>{'type': 'movie', 'id': id},
            'added_at': '2026-01-01T00:00:00Z',
          },
      ]),
      200,
    );
  }
  if (path == '/api/v1/titles/batch') {
    return http.Response(
      jsonEncode(<dynamic>[
        <String, dynamic>{'type': 'movie', 'id': 'm1', 'title': 'Alpha'},
      ]),
      200,
    );
  }
  if (req.method == 'PUT' && path.contains('/favorites/')) {
    _favorited.add(path.split('/favorites/').last);
    return http.Response('', 204);
  }
  // tokens and link-codes return empty arrays.
  return http.Response(jsonEncode(<dynamic>[]), 200);
}

final List<String> _watchlisted = <String>[];
final List<String> _favorited = <String>[];

Future<void> _pumpAccount(WidgetTester tester, String role) async {
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async => _route(req, role)),
  );
  await tester.pumpWidget(
    CapabilityScope(
      platform: 'desktop',
      measured: const ClientDecoding(
        video: <VideoCodecCap>[
          VideoCodecCap(codec: 'hevc', maxBitDepth: 10, smooth: true),
        ],
      ),
      child: ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          onGenerateRoute: (_) =>
              MaterialPageRoute<void>(builder: (_) => AccountPage(api: api)),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

double _cardTop(WidgetTester tester, Finder block) => tester
    .getTopLeft(
      find.descendant(of: block, matching: find.byType(Container)).first,
    )
    .dy;

void main() {
  setUp(() {
    SharedPreferences.setMockInitialValues(<String, Object>{});
    _watchlisted
      ..clear()
      ..add('m1');
    _favorited.clear();
  });

  testWidgets('favouriting from the watchlist fills the favorites block', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1400, 2000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await _pumpAccount(tester, 'user');

    expect(find.text(Strings.emptyFavorites), findsOneWidget);

    await tester.tap(
      find.byType(CatalogCardTile).first,
      buttons: kSecondaryButton,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.addFavorite));
    await tester.pumpAndSettle();

    expect(_favorited, <String>['m1']);
    expect(
      find.text(Strings.emptyFavorites),
      findsNothing,
      reason: 'the block below owns the list this action just joined',
    );
    expect(find.byType(CatalogCardTile), findsNWidgets(2));
  });

  testWidgets('a player sees only Profile and Library tabs', (
    WidgetTester tester,
  ) async {
    await _pumpAccount(tester, 'player');

    expect(find.text('Library'), findsOneWidget);
    expect(find.text('Devices & Access'), findsNothing);
    expect(find.byTooltip(Strings.changePassword), findsNothing);
    expect(find.byTooltip(Strings.editProfile), findsNothing);
    expect(find.text('API tokens'), findsNothing);
    expect(find.text('Sign out everywhere'), findsNothing);

    expect(
      find.text('Watchlist'),
      findsOneWidget,
      reason: 'Library is the landing tab, so the lists need no extra tap',
    );
  });

  testWidgets('a full user sees the Devices & Access tab and management', (
    WidgetTester tester,
  ) async {
    await _pumpAccount(tester, 'user');

    expect(find.text('Devices & Access'), findsOneWidget);

    await tester.tap(find.text('Profile'));
    await tester.pumpAndSettle();
    expect(
      find.widgetWithText(OutlinedButton, Strings.changePassword),
      findsOneWidget,
    );
    expect(
      find.widgetWithText(OutlinedButton, Strings.editProfile),
      findsOneWidget,
    );
    expect(find.text('Sign out everywhere'), findsOneWidget);

    await tester.tap(find.text('Devices & Access'));
    await tester.pumpAndSettle();
    expect(find.text('API tokens'), findsOneWidget);
  });

  testWidgets('an unselected tab adds no height to the page', (
    WidgetTester tester,
  ) async {
    // An IndexedStack sizes to its largest child, so Profile inherited the
    // Library tab's height and a phone scrolled a long way past the content.
    tester.view.physicalSize = const Size(390, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await _pumpAccount(tester, 'user');

    final double library = tester.getSize(find.byType(TabPanel)).height;

    await tester.tap(find.text('Profile'));
    await tester.pumpAndSettle();
    final double profile = tester.getSize(find.byType(TabPanel)).height;

    expect(
      profile,
      isNot(closeTo(library, 1)),
      reason: 'each tab has to be its own height, not the tallest one',
    );
  });

  testWidgets('a wide screen puts Profile and Playback support on one row', (
    WidgetTester tester,
  ) async {
    // Both blocks are short label and value lists, so stacking them full width
    // on a desktop window left most of the row empty.
    tester.view.physicalSize = const Size(1200, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await _pumpAccount(tester, 'user');
    await tester.tap(find.text(Strings.accountTabProfile));
    await tester.pumpAndSettle();

    final Offset profile = tester.getTopLeft(find.byType(ProfileBlock));
    final Offset playback = tester.getTopLeft(
      find.byType(PlaybackSupportBlock),
    );

    expect(profile.dy, closeTo(playback.dy, 1));
    expect(profile.dx, lessThan(playback.dx));
    expect(
      tester.getSize(find.byType(ProfileBlock)).width,
      closeTo(tester.getSize(find.byType(PlaybackSupportBlock)).width, 1),
      reason: 'the two halves have to match, not one wide and one narrow',
    );

    // Profile carries header buttons and Playback support does not, so without
    // a shared header height its card sat 23px lower than the one beside it.
    expect(
      _cardTop(tester, find.byType(ProfileBlock)),
      closeTo(_cardTop(tester, find.byType(PlaybackSupportBlock)), 1),
      reason: 'the two cards have to start at the same height',
    );
  });

  testWidgets('a narrow screen stacks Profile above Playback support', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(600, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await _pumpAccount(tester, 'user');
    await tester.tap(find.text(Strings.accountTabProfile));
    await tester.pumpAndSettle();

    expect(
      tester.getTopLeft(find.byType(ProfileBlock)).dy,
      lessThan(tester.getTopLeft(find.byType(PlaybackSupportBlock)).dy),
    );
  });

  testWidgets('a device says when it was last seen in words, not as sent', (
    WidgetTester tester,
  ) async {
    // The server sends ISO 8601 and the client owns the rendering.
    await _pumpAccount(tester, 'user');
    await tester.tap(find.text('Devices & Access'));
    await tester.pumpAndSettle();

    expect(find.textContaining('2026-08-17T09:24'), findsNothing);
    expect(
      find.text(Strings.lastSeen(sinceText('2026-08-17T09:24:11Z')!)),
      findsOneWidget,
    );
  });
}
