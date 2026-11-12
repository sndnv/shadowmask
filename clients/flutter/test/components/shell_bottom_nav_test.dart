import 'dart:convert';

import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/shell_bottom_nav.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_destination.dart';
import 'package:shadowmask/nav/nav_destinations.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

const SelfUser _admin = SelfUser(
  id: 'u1',
  username: 'pat',
  role: UserRole.admin,
);

const SelfUser _player = SelfUser(
  id: 'u2',
  username: 'sam',
  role: UserRole.player,
);

Future<({List<String> routes, List<String> calls})> _pump(
  WidgetTester tester, {
  required SelfUser? user,
  NavSection current = NavSection.movies,
}) async {
  final List<String> routes = <String>[];
  final List<String> calls = <String>[];
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      calls.add(req.url.path);
      return http.Response('{}', 200);
    }),
  );
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      onGenerateRoute: (RouteSettings settings) {
        routes.add(settings.name ?? '');
        return MaterialPageRoute<void>(
          settings: settings,
          builder: (_) => Scaffold(
            bottomNavigationBar: ShellBottomNav(
              api: api,
              current: current,
              user: user,
            ),
          ),
        );
      },
    ),
  );
  await tester.pumpAndSettle();
  return (routes: routes, calls: calls);
}

Finder _card() =>
    find.ancestor(of: find.text('pat'), matching: find.byType(Material)).first;

int _selected(WidgetTester tester) =>
    tester.widget<NavigationBar>(find.byType(NavigationBar)).selectedIndex;

void main() {
  testWidgets('the bar is four destinations and More, whoever is signed in', (
    WidgetTester tester,
  ) async {
    for (final SelfUser user in <SelfUser>[_admin, _player]) {
      await _pump(tester, user: user);

      expect(find.text(Strings.navigationHome), findsOneWidget);
      expect(find.text(Strings.navigationMovies), findsOneWidget);
      expect(find.text(Strings.navigationSeries), findsOneWidget);
      expect(find.text(Strings.navigationSearch), findsOneWidget);
      expect(find.text(Strings.navigationMore), findsOneWidget);
      expect(
        find.text(Strings.navigationCollections),
        findsNothing,
        reason: 'search earns a tab ahead of collections, for ${user.role}',
      );
    }
  });

  testWidgets('tapping a tab opens its section over Home alone', (
    WidgetTester tester,
  ) async {
    final ({List<String> routes, List<String> calls}) run = await _pump(
      tester,
      user: _admin,
    );
    final NavigatorState nav = tester.state<NavigatorState>(
      find.byType(Navigator),
    );
    for (final String page in <String>[
      '/home',
      '/title?id=7',
      '/season?id=3',
    ]) {
      nav.pushNamed(page);
      await tester.pumpAndSettle();
    }

    await tester.tap(find.text(Strings.navigationSeries));
    await tester.pumpAndSettle();

    expect(run.routes.last, '/series');
    expect(
      find.byType(ShellBottomNav, skipOffstage: false),
      findsNWidgets(3),
      reason: 'the root, Home and the section; the trail is gone',
    );
  });

  testWidgets('More carries the leftovers, the account and sign out', (
    WidgetTester tester,
  ) async {
    await _pump(tester, user: _admin);

    await tester.tap(find.text(Strings.navigationMore));
    await tester.pumpAndSettle();

    expect(find.text(Strings.navigationCollections), findsOneWidget);
    expect(find.text(Strings.navigationAdmin), findsOneWidget);
    expect(find.text('pat'), findsOneWidget);
    expect(find.byTooltip(Strings.signOut), findsOneWidget);
  });

  testWidgets(
    'the account and its sign out sit on one card, apart from Admin',
    (WidgetTester tester) async {
      await _pump(tester, user: _admin);

      await tester.tap(find.text(Strings.navigationMore));
      await tester.pumpAndSettle();

      final Finder card = _card();
      expect(
        tester.widget<Material>(card).color,
        Tokens.dark.surfaceAlt,
        reason: 'its own surface sets the account apart from the rows above',
      );
      expect(
        find.descendant(of: card, matching: find.byTooltip(Strings.signOut)),
        findsOneWidget,
      );
      expect(
        find.descendant(of: card, matching: find.text('P')),
        findsOneWidget,
      );
      expect(
        find.descendant(of: card, matching: find.text(Strings.navigationAdmin)),
        findsNothing,
      );
    },
  );

  testWidgets(
    'the account card is outlined and read as selected on the account page',
    (WidgetTester tester) async {
      final SemanticsHandle semantics = tester.ensureSemantics();
      for (final (NavSection section, Color border, bool selected)
          in <(NavSection, Color, bool)>[
            (NavSection.account, Tokens.dark.accent, true),
            (NavSection.movies, Tokens.dark.border, false),
          ]) {
        await tester.pumpWidget(const SizedBox());
        await _pump(tester, user: _admin, current: section);

        await tester.tap(find.text(Strings.navigationMore));
        await tester.pumpAndSettle();

        final Material card = tester.widget<Material>(_card());
        expect(
          (card.shape! as RoundedRectangleBorder).side.color,
          border,
          reason: 'on $section',
        );
        expect(
          tester.getSemantics(find.text('pat')),
          isSemantics(isSelected: selected),
          reason:
              'a screen reader hears the selection the outline shows, on $section',
        );
      }
      semantics.dispose();
    },
  );

  testWidgets('a signed out shell offers no account and no sign out', (
    WidgetTester tester,
  ) async {
    await _pump(tester, user: null);

    await tester.tap(find.text(Strings.navigationMore));
    await tester.pumpAndSettle();

    expect(find.text(Strings.navigationCollections), findsOneWidget);
    expect(find.text(Strings.navigationAdmin), findsNothing);
    expect(find.byTooltip(Strings.signOut), findsNothing);
  });

  testWidgets('choosing the account from More navigates there', (
    WidgetTester tester,
  ) async {
    final ({List<String> routes, List<String> calls}) run = await _pump(
      tester,
      user: _admin,
    );

    await tester.tap(find.text(Strings.navigationMore));
    await tester.pumpAndSettle();
    await tester.tap(find.text('pat'));
    await tester.pumpAndSettle();

    expect(run.routes.last, '/account');
  });

  testWidgets('signing out from More clears the stack', (
    WidgetTester tester,
  ) async {
    final ({List<String> routes, List<String> calls}) run = await _pump(
      tester,
      user: _admin,
    );

    await tester.tap(find.text(Strings.navigationMore));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip(Strings.signOut));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.signOut));
    await tester.pumpAndSettle();

    expect(run.calls, isEmpty);
    expect(run.routes.last, '/');
  });

  testWidgets('cancelling the sign out from More stays put', (
    WidgetTester tester,
  ) async {
    final ({List<String> routes, List<String> calls}) run = await _pump(
      tester,
      user: _admin,
    );
    final int before = run.routes.length;

    await tester.tap(find.text(Strings.navigationMore));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip(Strings.signOut));
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmSignOut), findsOneWidget);
    await tester.tap(find.text(Strings.cancel));
    await tester.pumpAndSettle();

    expect(run.calls, isEmpty);
    expect(run.routes.length, before);
    expect(find.text(Strings.confirmSignOut), findsNothing);
  });

  testWidgets('signing out from More revokes the linked device', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{});
    final List<String> calls = <String>[];
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        calls.add('${req.method} ${req.url.path}');
        return http.Response(
          jsonEncode(<String, dynamic>{
            'token': 'smk_abc',
            'device_id': 'dev-7',
          }),
          200,
        );
      }),
    );
    await api.redeemLinkCode('CODE', deviceName: 'd', platform: 'android');
    calls.clear();

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) => Scaffold(
            bottomNavigationBar: ShellBottomNav(
              api: api,
              current: NavSection.movies,
              user: _admin,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.navigationMore));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip(Strings.signOut));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.signOut));
    await tester.pumpAndSettle();

    expect(calls, contains('DELETE /api/v1/users/u1/devices/dev-7'));
  });

  testWidgets('the current section picks its own tab', (
    WidgetTester tester,
  ) async {
    await _pump(tester, user: _admin, current: NavSection.series);

    expect(_selected(tester), 2);
  });

  testWidgets('a section behind More selects More', (
    WidgetTester tester,
  ) async {
    await _pump(tester, user: _admin, current: NavSection.admin);

    expect(_selected(tester), 4);
  });

  testWidgets('a page belonging to no section still selects More', (
    WidgetTester tester,
  ) async {
    for (final NavSection section in <NavSection>[
      NavSection.account,
      NavSection.none,
    ]) {
      await _pump(tester, user: _admin, current: section);

      expect(
        _selected(tester),
        4,
        reason: 'NavigationBar needs a valid index, and More is the catch-all',
      );
    }
  });

  testWidgets('every destination is reachable from the bar or from More', (
    WidgetTester tester,
  ) async {
    await _pump(tester, user: _admin);
    final Set<String> onScreen = <String>{};

    for (final NavDestination d in destinationsFor(_admin)) {
      if (find.text(d.label).evaluate().isNotEmpty) {
        onScreen.add(d.label);
      }
    }
    await tester.tap(find.text(Strings.navigationMore));
    await tester.pumpAndSettle();
    for (final NavDestination d in destinationsFor(_admin)) {
      if (find.text(d.label).evaluate().isNotEmpty) {
        onScreen.add(d.label);
      }
    }

    expect(
      onScreen,
      destinationsFor(_admin).map((NavDestination d) => d.label).toSet(),
    );
  });
}
