import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/back_link.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Widget _app({required String initial}) => ThemeScope(
  variant: AppThemeVariant.dark,
  setVariant: (_) {},
  child: MaterialApp(
    theme: buildTheme(AppThemeVariant.dark),
    onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
      settings: settings,
      builder: (BuildContext context) => Scaffold(
        body: Column(
          children: <Widget>[
            const BackLink(),
            Text('at ${settings.name}'),
            TextButton(
              onPressed: () =>
                  Navigator.of(context).pushNamed(movieRoute('m1')),
              child: const Text('to movie'),
            ),
            TextButton(
              onPressed: () =>
                  Navigator.of(context).pushReplacementNamed(moviesRoute()),
              child: const Text('nav to movies'),
            ),
          ],
        ),
      ),
    ),
    onGenerateInitialRoutes: (String name) => <Route<dynamic>>[
      MaterialPageRoute<void>(
        settings: RouteSettings(name: name),
        builder: (BuildContext context) => Scaffold(
          body: Column(
            children: <Widget>[
              const BackLink(),
              Text('at $name'),
              TextButton(
                onPressed: () =>
                    Navigator.of(context).pushNamed(movieRoute('m1')),
                child: const Text('to movie'),
              ),
            ],
          ),
        ),
      ),
    ],
    initialRoute: initial,
  ),
);

void main() {
  testWidgets('a root page reached directly has no back link', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_app(initial: moviesRoute()));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.arrow_back), findsNothing);
  });

  testWidgets('a detail page pushed from a list gets a back link', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_app(initial: moviesRoute()));
    await tester.pumpAndSettle();

    await tester.tap(find.text('to movie'));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.arrow_back), findsOneWidget);

    await tester.tap(find.byIcon(Icons.arrow_back));
    await tester.pumpAndSettle();

    expect(find.text('at ${moviesRoute()}'), findsOneWidget);
  });

  testWidgets('a root page still has no back link on a deep stack', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_app(initial: moviesRoute()));
    await tester.pumpAndSettle();

    await tester.tap(find.text('to movie'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('nav to movies'));
    await tester.pumpAndSettle();

    expect(
      find.byIcon(Icons.arrow_back),
      findsNothing,
      reason: 'the nav bar replaces the top route, so the stack stays deep',
    );
  });

  testWidgets('a detail page reached directly has no back link', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_app(initial: movieRoute('m1')));
    await tester.pumpAndSettle();

    expect(
      find.byIcon(Icons.arrow_back),
      findsNothing,
      reason: 'a fresh load has no previous page to return to',
    );
  });

  test('a collection detail is not a root route', () {
    expect(isRootRoute(collectionsRoute()), isTrue);
    expect(isRootRoute(collectionRoute('c1')), isFalse);
    expect(isRootRoute(personRoute('p1')), isFalse);
    expect(isRootRoute(jobRoute('j1')), isFalse);
    expect(isRootRoute(adminJobsRoute()), isFalse);
    expect(isRootRoute(adminRoute()), isTrue);
    expect(isRootRoute(null), isFalse);
  });
}
