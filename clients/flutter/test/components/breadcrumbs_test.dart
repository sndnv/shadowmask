import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/crumb_strip.dart';
import 'package:shadowmask/components/crumbs_scope.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Future<void> _pump(WidgetTester tester, Widget child) async {
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(body: child),
      ),
    ),
  );
  await tester.pump();
}

void main() {
  testWidgets('publishes crumbs to the enclosing scope and renders nothing', (
    WidgetTester tester,
  ) async {
    final ValueNotifier<List<Crumb>> sink = ValueNotifier<List<Crumb>>(
      const <Crumb>[],
    );
    addTearDown(sink.dispose);
    await _pump(
      tester,
      CrumbsScope(
        crumbs: sink,
        child: Breadcrumbs(<Crumb>[
          Crumb(Strings.navigationMovies, route: '/movies'),
          const Crumb('Blade Runner'),
        ]),
      ),
    );
    await tester.pump();

    expect(find.byType(CrumbStrip), findsNothing);
    expect(sink.value.length, 3);
    expect(sink.value.first.label, Strings.navigationHome);
    expect(sink.value.last.label, 'Blade Runner');
  });

  testWidgets('renders inline when there is no scope', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      Breadcrumbs(<Crumb>[
        Crumb(Strings.navigationMovies, route: '/movies'),
        const Crumb('Blade Runner'),
      ]),
    );

    expect(find.byType(CrumbStrip), findsOneWidget);
    expect(find.text(Strings.navigationHome), findsOneWidget);
  });

  testWidgets('links are accent and bold, the last crumb is muted', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      CrumbStrip(<Crumb>[
        Crumb(Strings.navigationHome, route: '/home'),
        const Crumb('Blade Runner'),
      ]),
    );

    final Text link = tester.widget<Text>(find.text(Strings.navigationHome));
    final Text last = tester.widget<Text>(find.text('Blade Runner'));
    expect(link.style?.fontWeight, FontWeight.w600);
    expect(last.style?.fontWeight, FontWeight.w400);
    expect(link.style?.color, isNot(last.style?.color));
  });

  testWidgets('tapping the last crumb rebuilds the route it is already on', (
    WidgetTester tester,
  ) async {
    const String route = '/episode?id=e1&season=se1';
    final List<String?> built = <String?>[];

    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          initialRoute: route,
          onGenerateRoute: (RouteSettings settings) {
            built.add(settings.name);
            return MaterialPageRoute<void>(
              settings: settings,
              builder: (_) => Scaffold(
                body: CrumbStrip(<Crumb>[
                  Crumb(Strings.navigationSeries, route: '/series'),
                  const Crumb('Episode 1'),
                ]),
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(built.last, route);
    final int before = built.length;

    await tester.tap(find.text('Episode 1'));
    await tester.pumpAndSettle();

    expect(built.length, before + 1);
    expect(
      built.last,
      route,
      reason: 'the same route is rebuilt, so every page reloads its own data',
    );
  });

  testWidgets('the refreshable crumb keeps the plain last-crumb formatting', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      CrumbStrip(<Crumb>[
        Crumb(Strings.navigationHome, route: '/home'),
        const Crumb('Blade Runner'),
      ]),
    );

    final Text last = tester.widget<Text>(find.text('Blade Runner'));
    expect(last.style?.fontWeight, FontWeight.w400);
    expect(find.byTooltip(Strings.refreshThisPage), findsOneWidget);
  });

  _collapseTests();
}

List<Crumb> _episodeChain() => <Crumb>[
  Crumb(Strings.navigationHome, route: '/home'),
  Crumb(Strings.navigationSeries, route: '/series'),
  const Crumb('The Expanse', route: '/title?type=series&id=sh1'),
  const Crumb('Season 2', route: '/season?id=s1'),
  const Crumb('Godspeed'),
];

Future<void> _pumpAt(WidgetTester tester, double width) async {
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: SizedBox(width: width, child: CrumbStrip(_episodeChain())),
        ),
      ),
    ),
  );
  await tester.pump();
}

void _collapseTests() {
  testWidgets('a wide strip shows every level', (WidgetTester tester) async {
    await _pumpAt(tester, 900);

    expect(find.text('The Expanse'), findsOneWidget);
    expect(find.text('Season 2'), findsOneWidget);
    expect(find.text('…'), findsNothing);
  });

  testWidgets('a phone keeps the first crumb and the last two', (
    WidgetTester tester,
  ) async {
    await _pumpAt(tester, 304);

    expect(tester.takeException(), isNull);
    expect(find.text(Strings.navigationHome), findsOneWidget);
    expect(find.text('Season 2'), findsOneWidget);
    expect(find.text('Godspeed'), findsOneWidget);
    // The middle of the chain moves behind the overflow control.
    expect(find.text('The Expanse'), findsNothing);
    expect(find.text(Strings.navigationSeries), findsNothing);
    expect(find.text('…'), findsOneWidget);
  });

  testWidgets('the overflow control reveals the levels it hides', (
    WidgetTester tester,
  ) async {
    await _pumpAt(tester, 304);

    await tester.tap(find.text('…'));
    await tester.pumpAndSettle();

    expect(find.text('The Expanse'), findsOneWidget);
    expect(find.text(Strings.navigationSeries), findsOneWidget);
  });

  testWidgets('picking a hidden level navigates to it', (
    WidgetTester tester,
  ) async {
    String? went;
    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          onGenerateRoute: (RouteSettings settings) {
            if (settings.name != null && settings.name != '/') {
              went = settings.name;
            }
            return MaterialPageRoute<void>(
              builder: (_) => Scaffold(
                body: SizedBox(width: 304, child: CrumbStrip(_episodeChain())),
              ),
            );
          },
        ),
      ),
    );
    await tester.pump();

    await tester.tap(find.text('…'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('The Expanse'));
    await tester.pumpAndSettle();

    expect(went, '/title?type=series&id=sh1');
  });

  testWidgets('the overflow control names itself for a screen reader', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pumpAt(tester, 304);

    expect(find.byTooltip(Strings.hiddenLevels), findsOneWidget);
    expect(
      tester.getSemantics(find.text('…')).label,
      contains(Strings.hiddenLevels),
    );
    semantics.dispose();
  });

  testWidgets('a short chain never collapses', (WidgetTester tester) async {
    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: Scaffold(
            body: SizedBox(
              width: 304,
              child: CrumbStrip(<Crumb>[
                Crumb(Strings.navigationHome, route: '/home'),
                Crumb(Strings.navigationSeries, route: '/series'),
                const Crumb('The Expanse'),
              ]),
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    expect(find.text('…'), findsNothing);
    expect(find.text(Strings.navigationSeries), findsOneWidget);
  });
}
