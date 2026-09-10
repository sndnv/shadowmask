import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/components/brand_mark.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/crumb_strip.dart';
import 'package:shadowmask/components/shell_bottom_nav.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_destination.dart';
import 'package:shadowmask/nav/nav_destinations.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/default/immersive_scope.dart';
import 'package:shadowmask/pages/default/shell_scaffold.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';

ApiClient _api() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request _) async => http.Response('{}', 200)),
);

Widget _host(Widget body, {bool fullWidth = false, bool fitViewport = false}) =>
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: ShellScaffold(
          api: _api(),
          current: NavSection.movies,
          fullWidth: fullWidth,
          fitViewport: fitViewport,
          body: body,
        ),
      ),
    );

const Widget _body = Column(
  key: Key('body'),
  crossAxisAlignment: CrossAxisAlignment.start,
  children: <Widget>[Text('narrow')],
);

Future<double> _bodyWidth(WidgetTester tester, {bool fullWidth = false}) async {
  tester.view.physicalSize = const Size(1600, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(_host(_body, fullWidth: fullWidth));
  return tester.getSize(find.byKey(const Key('body'))).width;
}

void main() {
  testWidgets('the username is the account link and the bar drops Account', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
            builder: (_) => settings.name == '/account'
                ? const Text('account page')
                : ShellScaffold(
                    api: _api(),
                    current: NavSection.movies,
                    user: const SelfUser(
                      id: 'u1',
                      username: 'pat',
                      role: UserRole.user,
                    ),
                    body: _body,
                  ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.navigationAccount), findsNothing);
    expect(find.text('pat'), findsOneWidget);

    await tester.tap(find.text('pat'));
    await tester.pumpAndSettle();

    expect(find.text('account page'), findsOneWidget);
  });

  testWidgets(
    'the body fills the shell width even when its content is narrow',
    (WidgetTester tester) async {
      expect(await _bodyWidth(tester), greaterThan(400));
    },
  );

  testWidgets('the body column fits a full row of six poster cards', (
    WidgetTester tester,
  ) async {
    const double row = 6 * kPosterCardWidth + 5 * Space.s4;

    expect(await _bodyWidth(tester), greaterThanOrEqualTo(row));
    expect(await _bodyWidth(tester), lessThan(row + kPosterCardWidth));
  });

  testWidgets('the card is capped for viewers and full width for admin', (
    WidgetTester tester,
  ) async {
    final double capped = await _bodyWidth(tester);
    expect(capped, lessThanOrEqualTo(Breakpoints.lg));

    final double full = await _bodyWidth(tester, fullWidth: true);
    expect(full, greaterThan(Breakpoints.lg));
  });

  testWidgets('the page scrolls while the nav stays pinned', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      _host(
        const Column(
          key: Key('body'),
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[SizedBox(height: 3000, child: Text('tall'))],
        ),
      ),
    );

    final double navBefore = tester.getTopLeft(find.byType(BrandMark)).dy;
    final double bodyBefore = tester
        .getTopLeft(find.byKey(const Key('body')))
        .dy;

    await tester.drag(find.byType(CustomScrollView), const Offset(0, -300));
    await tester.pumpAndSettle();

    expect(tester.getTopLeft(find.byType(BrandMark)).dy, navBefore);
    expect(
      tester.getTopLeft(find.byKey(const Key('body'))).dy,
      lessThan(bodyBefore - 200),
    );
  });

  testWidgets('a short body still fills the viewport height', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(_host(_body));

    final ScrollableState scrollable = tester.state<ScrollableState>(
      find.byType(Scrollable).first,
    );
    expect(scrollable.position.maxScrollExtent, 0);
  });

  testWidgets('fitViewport keeps the nav and stops the page scrolling', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      _host(const SizedBox(key: Key('body'), height: 3000), fitViewport: true),
    );
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.byType(BrandMark), findsOneWidget);

    final ScrollableState scrollable = tester.state<ScrollableState>(
      find.byType(Scrollable).first,
    );
    expect(scrollable.position.maxScrollExtent, 0);
    expect(
      tester.getBottomLeft(find.byKey(const Key('body'))).dy,
      lessThanOrEqualTo(700),
    );

    await tester.drag(find.byType(CustomScrollView), const Offset(0, -300));
    await tester.pumpAndSettle();

    expect(scrollable.position.pixels, 0);
  });

  testWidgets('a page backdrop is painted behind the shell', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(_body));
    await tester.pump();
    expect(find.byType(Image), findsNothing);

    await tester.pumpWidget(
      _host(
        const PageBackdrop(
          artwork: Artwork(
            backdrops: <ImageSet>[
              ImageSet(base: '/artwork/1', widths: <int>[1920]),
            ],
          ),
          imageBase: 'http://test',
        ),
      ),
    );
    await tester.pump();

    expect(find.byType(Image), findsWidgets);
  });

  testWidgets('breadcrumbs render in the body, under the pinned nav', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        Breadcrumbs(<Crumb>[
          Crumb(Strings.navigationMovies, route: '/movies'),
          const Crumb('Blade Runner'),
        ]),
      ),
    );
    await tester.pump();

    expect(find.byType(CrumbStrip), findsOneWidget);
    final double navBottom = tester.getBottomLeft(find.byType(BrandMark)).dy;
    expect(
      tester.getTopLeft(find.byType(CrumbStrip)).dy,
      greaterThan(navBottom),
    );
  });

  testWidgets('immersive drops the nav and gives the body the whole viewport', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    late ValueNotifier<bool> immersive;
    await tester.pumpWidget(
      _host(
        Builder(
          builder: (BuildContext context) {
            immersive = ImmersiveScope.of(context)!;
            return ValueListenableBuilder<bool>(
              valueListenable: immersive,
              builder: (BuildContext context, bool on, Widget? _) => on
                  ? const SizedBox.expand(key: Key('stage'))
                  : const SizedBox(key: Key('stage'), height: 200),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byType(BrandMark), findsOneWidget);
    expect(tester.takeException(), isNull);

    immersive.value = true;
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.byType(BrandMark), findsNothing);
    final Size stage = tester.getSize(find.byKey(const Key('stage')));
    expect(stage.height, 900);
    expect(stage.width, 1600);

    immersive.value = false;
    await tester.pumpAndSettle();

    expect(find.byType(BrandMark), findsOneWidget);
  });

  testWidgets('the tab bar belongs to phones only', (
    WidgetTester tester,
  ) async {
    await _navAt(tester, Breakpoints.sm - 1);
    expect(find.byType(ShellBottomNav), findsOneWidget);

    await _navAt(tester, Breakpoints.sm);
    expect(find.byType(ShellBottomNav), findsNothing);
  });

  testWidgets('a phone has no top bar at all, only the tab bar', (
    WidgetTester tester,
  ) async {
    await _navAt(tester, 390);

    expect(find.byType(ShellBottomNav), findsOneWidget);
    expect(find.byType(BrandMark), findsNothing);
    expect(
      find.text(Strings.signOut),
      findsNothing,
      reason: 'sign out moved into the More sheet',
    );
    expect(find.text('pat'), findsNothing);
  });

  testWidgets('dropping the top bar gives the body that height back', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(390, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(_host(const SizedBox(key: Key('tall'))));
    await tester.pumpAndSettle();

    expect(
      tester.getTopLeft(find.byKey(const Key('tall'))).dy,
      lessThan(kShellHeaderHeight),
      reason: 'the body starts at the top, not below a header that is gone',
    );
  });

  testWidgets('going immersive takes the tab bar with it', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(390, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    late ValueNotifier<bool> immersive;
    await tester.pumpWidget(
      _host(
        Builder(
          builder: (BuildContext context) {
            immersive = ImmersiveScope.of(context)!;
            return const SizedBox(height: 200);
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(ShellBottomNav), findsOneWidget);

    immersive.value = true;
    await tester.pumpAndSettle();

    expect(find.byType(ShellBottomNav), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('the immersive body element survives the transition', (
    WidgetTester tester,
  ) async {
    late ValueNotifier<bool> immersive;
    await tester.pumpWidget(
      _host(
        Builder(
          builder: (BuildContext context) {
            immersive = ImmersiveScope.of(context)!;
            return const _Counter(key: Key('counter'));
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    final _CounterState before = tester.state<_CounterState>(
      find.byKey(const Key('counter')),
    );
    before.bump();

    immersive.value = true;
    await tester.pumpAndSettle();

    expect(
      identical(
        tester.state<_CounterState>(find.byKey(const Key('counter'))),
        before,
      ),
      isTrue,
    );
    expect(before.count, 1);
  });

  testWidgets('switching theme repaints the nav, not just the body', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    late StateSetter setOuter;
    AppThemeVariant variant = AppThemeVariant.dark;
    await tester.pumpWidget(
      StatefulBuilder(
        builder: (BuildContext context, StateSetter setState) {
          setOuter = setState;
          return ThemeScope(
            variant: variant,
            setVariant: (_) {},
            child: MaterialApp(
              theme: buildTheme(variant),
              home: ShellScaffold(
                api: _api(),
                current: NavSection.movies,
                body: const Text('body'),
              ),
            ),
          );
        },
      ),
    );
    await tester.pumpAndSettle();

    Color navFill() =>
        (tester
                    .widget<Container>(
                      find
                          .descendant(
                            of: find.byType(SliverPersistentHeader),
                            matching: find.byType(Container),
                          )
                          .first,
                    )
                    .decoration!
                as BoxDecoration)
            .color!;

    expect(navFill(), Tokens.dark.surface);

    setOuter(() => variant = AppThemeVariant.light);
    await tester.pumpAndSettle();

    expect(
      navFill(),
      Tokens.light.surface,
      reason:
          'shouldRebuild ignored the theme, so the nav kept the old colours',
    );
  });

  testWidgets('dismissing a dialog leaves the scroll position alone', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      _host(
        Column(
          children: <Widget>[
            for (int i = 0; i < 40; i++)
              SizedBox(height: 80, child: Text('row $i')),
          ],
        ),
      ),
    );
    await tester.pumpAndSettle();

    final ScrollableState scrollable = tester.state<ScrollableState>(
      find.byType(Scrollable).first,
    );
    scrollable.position.jumpTo(900);
    await tester.pumpAndSettle();
    expect(scrollable.position.pixels, 900);

    final BuildContext context = tester.element(find.text('row 0'));
    unawaited(
      showDialog<void>(
        context: context,
        builder: (BuildContext _) => const AlertDialog(content: Text('hello')),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('hello'), findsOneWidget);

    Navigator.of(context, rootNavigator: true).pop();
    await tester.pumpAndSettle();

    expect(
      scrollable.position.pixels,
      900,
      reason: 'restoring focus must not scroll the page back to the top',
    );
  });

  _responsiveNavTests();
}

const SelfUser _admin = SelfUser(
  id: 'u1',
  username: 'pat',
  role: UserRole.admin,
);

Future<void> _navAt(
  WidgetTester tester,
  double width, {
  SelfUser? user = _admin,
}) async {
  tester.view.physicalSize = Size(width, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) => settings.name == null || settings.name == '/'
              ? ShellScaffold(
                  api: _api(),
                  current: NavSection.movies,
                  user: user,
                  body: _body,
                )
              : Text('at ${settings.name}'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

int _inBar(WidgetTester tester) => destinationsFor(
  _admin,
).where((NavDestination d) => find.text(d.label).evaluate().isNotEmpty).length;

void _responsiveNavTests() {
  test('everything fits when there is room for it', () {
    expect(fittingNavItems(available: 1000, widths: <double>[80, 80, 80]), 3);
  });

  test('an exact fit keeps every item and needs no overflow', () {
    expect(fittingNavItems(available: 240, widths: <double>[80, 80, 80]), 3);
  });

  test('the overflow button claims its own space before any item does', () {
    const List<double> widths = <double>[80, 80, 80, 80];

    expect(fittingNavItems(available: 250, widths: widths), 2);
    expect(
      fittingNavItems(available: 250, widths: widths, overflowWidth: 0),
      3,
    );
  });

  test('items leave one at a time as the space shrinks', () {
    const List<double> widths = <double>[80, 80, 80, 80];
    final List<int> counts = <int>[
      for (double w = 400; w >= 40; w -= 20)
        fittingNavItems(available: w, widths: widths),
    ];

    expect(counts.first, 4);
    expect(counts.last, 0);
    for (int i = 1; i < counts.length; i++) {
      expect(
        counts[i - 1] - counts[i],
        inInclusiveRange(0, 1),
        reason: 'count jumped from ${counts[i - 1]} to ${counts[i]}',
      );
    }
  });

  testWidgets('a wide window lays every destination out in the bar', (
    WidgetTester tester,
  ) async {
    await _navAt(tester, 1600);

    expect(find.text(Strings.navigationCollections), findsOneWidget);
    expect(find.text(Strings.navigationAdmin), findsOneWidget);
    expect(find.byIcon(Icons.more_horiz), findsNothing);
  });

  testWidgets('narrowing moves items into the overflow one at a time', (
    WidgetTester tester,
  ) async {
    final int total = destinationsFor(_admin).length;
    final List<int> seen = <int>[];
    // below sm the top bar has no destinations at all; the tab bar has them
    for (double width = 1600; width >= Breakpoints.sm; width -= 50) {
      await _navAt(tester, width);
      seen.add(_inBar(tester));
    }

    expect(seen.first, total, reason: 'not all shown when wide: $seen');
    expect(seen.last, lessThan(total), reason: 'nothing overflowed: $seen');

    for (int i = 1; i < seen.length; i++) {
      expect(
        seen[i],
        lessThanOrEqualTo(seen[i - 1]),
        reason: 'the bar grew as the window shrank: $seen',
      );
    }

    final Set<int> partial = seen.where((int n) => n > 0 && n < total).toSet();
    expect(
      partial.length,
      greaterThanOrEqualTo(3),
      reason: 'it snapped instead of shedding one at a time: $seen',
    );
  });

  testWidgets('sign out stays in the bar at every width that has a bar', (
    WidgetTester tester,
  ) async {
    for (final double width in <double>[Breakpoints.sm, 700, 1600]) {
      await _navAt(tester, width);
      expect(find.text(Strings.signOut), findsOneWidget, reason: 'at $width');
      expect(find.text('pat'), findsOneWidget, reason: 'at $width');
    }
  });

  testWidgets('every destination stays reachable once overflowed', (
    WidgetTester tester,
  ) async {
    await _navAt(tester, 600);
    expect(find.byIcon(Icons.more_horiz), findsOneWidget);

    await tester.tap(find.byIcon(Icons.more_horiz));
    await tester.pumpAndSettle();

    for (final NavDestination d in destinationsFor(_admin)) {
      expect(
        find.text(d.label),
        findsOneWidget,
        reason: '${d.label} unreached',
      );
    }
  });

  testWidgets('choosing an overflowed destination navigates to it', (
    WidgetTester tester,
  ) async {
    await _navAt(tester, 600);

    await tester.tap(find.byIcon(Icons.more_horiz));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.navigationAdmin));
    await tester.pumpAndSettle();

    expect(find.text('at /admin'), findsOneWidget);
  });

  testWidgets('the nav never scrolls sideways at any width', (
    WidgetTester tester,
  ) async {
    for (final double width in <double>[420, 600, 900, 1600]) {
      await _navAt(tester, width);

      final Finder horizontal = find.descendant(
        of: find.byType(SliverPersistentHeader),
        matching: find.byWidgetPredicate(
          (Widget w) =>
              w is SingleChildScrollView &&
              w.scrollDirection == Axis.horizontal,
        ),
      );
      expect(horizontal, findsNothing, reason: 'nav scrolls at $width');
      expect(tester.takeException(), isNull, reason: 'overflowed at $width');
    }
  });

  testWidgets('a viewer never sees an admin entry, overflowed or not', (
    WidgetTester tester,
  ) async {
    await _navAt(
      tester,
      600,
      user: const SelfUser(id: 'u2', username: 'sam', role: UserRole.user),
    );

    expect(find.text(Strings.navigationAdmin), findsNothing);

    await tester.tap(find.byIcon(Icons.more_horiz));
    await tester.pumpAndSettle();

    expect(find.text(Strings.navigationAdmin), findsNothing);
  });

  testWidgets('every label in the bar is set from one type style', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: ShellScaffold(
            api: _api(),
            current: NavSection.movies,
            user: const SelfUser(
              id: 'u1',
              username: 'pat',
              role: UserRole.user,
            ),
            body: _body,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final TextButton signOut = tester.widget<TextButton>(
      find.widgetWithText(TextButton, Strings.signOut),
    );
    expect(
      signOut.style?.textStyle?.resolve(<WidgetState>{}),
      kNavItemText,
      reason:
          'a size of its own puts the sign out baseline off the row, because '
          'boxes of different heights centre to different baselines',
    );
    expect(
      tester.widget<Text>(find.text('/')).style?.fontSize,
      kNavItemText.fontSize,
      reason: 'without a size the separator inherits the theme body size',
    );
  });

  testWidgets('the top bar clears the status bar and the side cutout', (
    WidgetTester tester,
  ) async {
    const EdgeInsets safe = EdgeInsets.fromLTRB(0, 44, 0, 34);
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: MediaQuery(
            data: const MediaQueryData(size: Size(1200, 800), padding: safe),
            child: ShellScaffold(
              api: _api(),
              current: NavSection.movies,
              body: _body,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      tester.getRect(find.byType(BrandMark)).top,
      greaterThanOrEqualTo(safe.top),
      reason: 'the brand sits inside the bar, which sits below the status bar',
    );
  });

  testWidgets('a phone with no top bar still keeps its body off the clock', (
    WidgetTester tester,
  ) async {
    const EdgeInsets safe = EdgeInsets.fromLTRB(0, 44, 0, 34);
    tester.view.physicalSize = const Size(360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: MediaQuery(
            data: const MediaQueryData(size: Size(360, 800), padding: safe),
            child: ShellScaffold(
              api: _api(),
              current: NavSection.movies,
              body: _body,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      tester.getRect(find.byKey(const Key('body'))).top,
      greaterThanOrEqualTo(safe.top),
    );
  });
}

class _Counter extends StatefulWidget {
  const _Counter({super.key});

  @override
  State<_Counter> createState() => _CounterState();
}

class _CounterState extends State<_Counter> {
  int count = 0;

  void bump() => count++;

  @override
  Widget build(BuildContext context) => const SizedBox(height: 200);
}
