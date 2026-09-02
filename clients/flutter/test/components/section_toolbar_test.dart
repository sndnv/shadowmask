import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/components/section_toolbar.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Widget _host(Widget child) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(body: child),
);

const List<Genre> _genres = <Genre>[
  Genre(id: 'action', name: 'Action'),
  Genre(id: 'comedy', name: 'Comedy'),
  Genre(id: 'drama', name: 'Drama'),
];

void main() {
  testWidgets('genre dropdown opens a themed checklist and applies', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        SectionToolbar(
          basePath: '/movies',
          genres: _genres,
          sort: 'added_at',
          order: 'desc',
          selectedGenres: const <String>[],
        ),
      ),
    );

    expect(find.text('Genres'), findsOneWidget);

    await tester.tap(find.text('Genres'));
    await tester.pumpAndSettle();

    expect(find.text('Action'), findsOneWidget);
    expect(find.text('Comedy'), findsOneWidget);
    expect(find.text(Strings.applyAction), findsOneWidget);
  });

  testWidgets('the button shows the selected count', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        SectionToolbar(
          basePath: '/movies',
          genres: _genres,
          sort: 'added_at',
          order: 'desc',
          selectedGenres: const <String>['Action', 'Drama'],
        ),
      ),
    );

    expect(find.text('Genres'), findsOneWidget);
    expect(find.text('2'), findsOneWidget);
  });

  testWidgets('a page size in the url survives a sort change', (
    WidgetTester tester,
  ) async {
    String? went;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (RouteSettings settings) {
          if (settings.name != null && settings.name != '/') {
            went = settings.name;
          }
          return MaterialPageRoute<void>(
            builder: (_) => const Scaffold(
              body: SectionToolbar(
                basePath: '/movies',
                genres: _genres,
                sort: 'added_at',
                order: 'desc',
                selectedGenres: <String>[],
                limit: 3,
              ),
            ),
          );
        },
      ),
    );

    await tester.tap(find.text('Added'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Title').last);
    await tester.pumpAndSettle();

    expect(went, contains('limit=3'));
    expect(went, contains('sort=title'));
  });

  testWidgets('a trailing action is pushed to the right of the filters', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      _host(
        SectionToolbar(
          basePath: '/movies',
          genres: _genres,
          sort: 'added_at',
          order: 'desc',
          selectedGenres: const <String>[],
          trailing: IconButton(
            key: const Key('trailing'),
            onPressed: () {},
            icon: const Icon(Icons.shuffle),
          ),
        ),
      ),
    );

    final Rect row = tester.getRect(find.byType(SectionToolbar));
    final Rect genres = tester.getRect(find.text('Genres'));
    final Rect trailing = tester.getRect(find.byKey(const Key('trailing')));

    expect(
      trailing.center.dx,
      greaterThan(genres.center.dx),
      reason: 'the action sits after the filters, not among them',
    );
    expect(
      trailing.right,
      moreOrLessEquals(row.right, epsilon: 1),
      reason: 'it is pinned to the right edge of the same row',
    );
    expect(
      trailing.center.dy,
      moreOrLessEquals(genres.center.dy, epsilon: 12),
      reason: 'it stays on the filter row rather than wrapping below it',
    );
  });

  testWidgets('a narrow toolbar never forces the action onto its own line', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const SizedBox(
          width: 304,
          child: SectionToolbar(
            basePath: '/movies',
            genres: _genres,
            sort: 'added_at',
            order: 'desc',
            selectedGenres: <String>[],
            trailing: IconButton(
              key: Key('trailing'),
              onPressed: _noop,
              icon: Icon(Icons.shuffle),
            ),
          ),
        ),
      ),
    );

    expect(tester.takeException(), isNull);
    final Rect sort = tester.getRect(find.text(Strings.sortAdded));
    final Rect trailing = tester.getRect(find.byKey(const Key('trailing')));

    // The test font draws every glyph 14px wide, so the filters wrap here even
    // though they fit a real 304px body. What must hold at any width is that
    // the action shares the first run rather than being pushed below.
    expect(trailing.center.dy, moreOrLessEquals(sort.center.dy, epsilon: 12));
  });

  testWidgets('a wrapped filter block keeps the action at the top', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const SizedBox(
          width: 200,
          child: SectionToolbar(
            basePath: '/movies',
            genres: _genres,
            sort: 'added_at',
            order: 'desc',
            selectedGenres: <String>[],
            trailing: IconButton(
              key: Key('trailing'),
              onPressed: _noop,
              icon: Icon(Icons.shuffle),
            ),
          ),
        ),
      ),
    );

    final Rect sort = tester.getRect(find.text(Strings.sortAdded));
    final Rect genres = tester.getRect(find.text(Strings.genresLabel));
    final Rect trailing = tester.getRect(find.byKey(const Key('trailing')));

    expect(
      genres.top,
      greaterThan(sort.bottom),
      reason: 'the filters genuinely need a second run at this width',
    );
    expect(
      trailing.center.dy,
      lessThan(genres.top),
      reason:
          'the action stays on the first run instead of centring itself '
          'against the whole two-run block',
    );
  });

  testWidgets('the sort and order controls carry labels', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const SectionToolbar(
          basePath: '/movies',
          genres: _genres,
          sort: 'added_at',
          order: 'desc',
          selectedGenres: <String>[],
        ),
      ),
    );

    expect(find.byTooltip(Strings.sortLabel), findsOneWidget);
    expect(
      find.byTooltip(Strings.orderTooltip(Strings.orderDescending)),
      findsOneWidget,
      reason: 'the arrow alone does not say which way it sorts',
    );
  });

  testWidgets('the order control is an arrow that flips the order', (
    WidgetTester tester,
  ) async {
    String? went;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (RouteSettings settings) {
          if (settings.name != null && settings.name != '/') {
            went = settings.name;
          }
          return MaterialPageRoute<void>(
            builder: (_) => const Scaffold(
              body: SectionToolbar(
                basePath: '/movies',
                genres: _genres,
                sort: 'added_at',
                order: 'asc',
                selectedGenres: <String>[],
              ),
            ),
          );
        },
      ),
    );

    expect(find.text(Strings.orderAscending), findsNothing);
    expect(find.byIcon(Icons.arrow_upward), findsOneWidget);

    await tester.tap(find.byIcon(Icons.arrow_upward));
    await tester.pumpAndSettle();

    expect(went, contains('order=desc'));
  });

  testWidgets('the order control fits the height of the dropdowns', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const SectionToolbar(
          basePath: '/movies',
          genres: _genres,
          sort: 'added_at',
          order: 'asc',
          selectedGenres: <String>[],
        ),
      ),
    );

    expect(
      tester.getSize(find.byType(ToggleButton)).height,
      tester.getSize(find.byType(MenuField).first).height,
      reason: 'the toggle sits between two dropdowns and must line up',
    );
  });
}

void _noop() {}
