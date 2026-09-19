import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/components/section_toolbar.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Widget _host(Widget child) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(body: child),
);

const List<Library> _libraries = <Library>[
  Library(
    id: 'lib1',
    name: 'Films',
    kind: LibraryKind.movie,
    watcher: WatcherStrategy.manual,
    createdAt: '',
    updatedAt: '',
  ),
  Library(
    id: 'lib2',
    name: 'Shows',
    kind: LibraryKind.tv,
    watcher: WatcherStrategy.manual,
    createdAt: '',
    updatedAt: '',
  ),
];

const List<Genre> _genres = <Genre>[
  Genre(id: 'action', name: 'Action'),
  Genre(id: 'comedy', name: 'Comedy'),
  Genre(id: 'drama', name: 'Drama'),
];

Future<void> _pumpPhone(
  WidgetTester tester, {
  List<String> selected = const <String>[],
  void Function(String route)? onRoute,
}) async {
  tester.view.physicalSize = const Size(390, 844);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      onGenerateRoute: (RouteSettings settings) {
        if (settings.name != null && settings.name != '/') {
          onRoute?.call(settings.name!);
        }
        return MaterialPageRoute<void>(
          builder: (_) => Scaffold(
            body: SectionToolbar(
              basePath: '/movies',
              genres: _genres,
              sort: 'added_at',
              order: 'desc',
              selectedGenres: selected,
              libraries: _libraries,
            ),
          ),
        );
      },
    ),
  );
  await tester.pumpAndSettle();
}

const String _sortField = '${Strings.sortLabel}: ${Strings.sortAdded}';
const String _genresField = '${Strings.genresLabel}: ${Strings.filterAll}';
const String _genresChosen = '${Strings.genresLabel}:';
const String _libraryField = '${Strings.libraryLabel}: ${Strings.filterAll}';

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

    expect(find.text(_genresField), findsOneWidget);

    await tester.tap(find.text(_genresField));
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

    expect(find.text(_genresChosen), findsOneWidget);
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

    await tester.tap(find.text(_sortField));
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
    final Rect genres = tester.getRect(find.text(_genresField));
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
    final Rect sort = tester.getRect(find.text(_sortField));
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

    final Rect sort = tester.getRect(find.text(_sortField));
    final Rect genres = tester.getRect(find.text(_genresField));
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

    expect(
      find.text(_sortField),
      findsOneWidget,
      reason: 'a bare value never said which control it belonged to',
    );
    expect(
      find.text(Strings.orderTooltip(Strings.orderDescending)),
      findsOneWidget,
      reason: 'the arrow alone does not say which way it sorts',
    );
    expect(find.text(_genresField), findsOneWidget);
    expect(
      find.byTooltip(Strings.sortLabel),
      findsNothing,
      reason: 'the label is on screen now, so the tooltip only repeats it',
    );
  });

  testWidgets('a phone shows one Filters button instead of four controls', (
    WidgetTester tester,
  ) async {
    await _pumpPhone(tester);

    expect(find.text(Strings.filtersHeading), findsOneWidget);
    expect(find.text(_sortField), findsNothing);
    expect(find.text(_genresField), findsNothing);
    expect(find.text(_libraryField), findsNothing);
    expect(
      find.text(Strings.orderTooltip(Strings.orderDescending)),
      findsNothing,
    );
  });

  testWidgets('the button counts the filters that are actually on', (
    WidgetTester tester,
  ) async {
    await _pumpPhone(tester, selected: const <String>['Action', 'Drama']);

    expect(
      find.text('2'),
      findsOneWidget,
      reason: 'the sheet is shut, so the count is all it can say',
    );

    await _pumpPhone(tester);
    expect(
      find.text('0'),
      findsNothing,
      reason: 'no filters is a plain button, not a zero',
    );
  });

  testWidgets('the sheet opens with every control and applies them at once', (
    WidgetTester tester,
  ) async {
    String? went;
    await _pumpPhone(tester, onRoute: (String r) => went = r);

    await tester.tap(find.text(Strings.filtersHeading));
    await tester.pumpAndSettle();

    expect(find.text(Strings.sortLabel), findsOneWidget);
    expect(find.text(Strings.orderLabel), findsOneWidget);
    expect(find.text(Strings.genresLabel), findsOneWidget);
    expect(find.text(Strings.libraryLabel), findsOneWidget);

    await tester.tap(find.text(Strings.filterAll).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Action'));
    await tester.pumpAndSettle();
    expect(
      find.text(Strings.applyAction),
      findsOneWidget,
      reason: 'the sheet owns the only Apply; the genre list commits as you go',
    );

    await tester.tap(find.text(Strings.applyAction));
    await tester.pumpAndSettle();

    expect(went, contains('genres=Action'));
    expect(went, contains('sort=added_at'));
  });

  testWidgets('closing the sheet without applying changes nothing', (
    WidgetTester tester,
  ) async {
    String? went;
    await _pumpPhone(tester, onRoute: (String r) => went = r);

    await tester.tap(find.text(Strings.filtersHeading));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.filterAll).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Action'));
    await tester.pumpAndSettle();
    Navigator.of(tester.element(find.text(Strings.applyAction))).pop();
    await tester.pumpAndSettle();

    expect(went, isNull, reason: 'dismissing is not the same as applying');
  });

  testWidgets('every browse control leads with its own glyph', (
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
          libraries: _libraries,
        ),
      ),
    );

    expect(find.byIcon(Icons.sort), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(ToggleButton),
        matching: find.byIcon(Icons.expand_more),
      ),
      findsOneWidget,
      reason: 'every dropdown caret is this glyph too, so scope it',
    );
    expect(find.byIcon(Icons.filter_alt_outlined), findsOneWidget);
    expect(find.byIcon(Icons.folder_outlined), findsOneWidget);
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

    expect(
      find.text(Strings.orderTooltip(Strings.orderAscending)),
      findsOneWidget,
      reason: 'the control names itself and the direction it is pointing',
    );
    expect(find.byIcon(Icons.expand_less), findsOneWidget);

    await tester.tap(find.byIcon(Icons.expand_less));
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

  testWidgets('no library control when the server reports no libraries', (
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

    expect(find.byTooltip(Strings.libraryLabel), findsNothing);
    expect(find.text(_libraryField), findsNothing);
  });

  testWidgets('the library control names the libraries and All', (
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
          libraries: _libraries,
        ),
      ),
    );

    expect(
      find.text(_libraryField),
      findsOneWidget,
      reason: 'with no library chosen the control reads as unfiltered',
    );

    await tester.tap(find.text(_libraryField));
    await tester.pumpAndSettle();

    expect(find.text('Films'), findsOneWidget);
    expect(find.text('Shows'), findsOneWidget);
  });

  testWidgets('choosing a library puts it in the url and keeps the sort', (
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
                sort: 'title',
                order: 'desc',
                selectedGenres: <String>[],
                libraries: _libraries,
              ),
            ),
          );
        },
      ),
    );

    await tester.tap(find.text(_libraryField));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Shows').last);
    await tester.pumpAndSettle();

    expect(went, contains('library=lib2'));
    expect(went, contains('sort=title'));
  });

  testWidgets('choosing All clears the library rather than keeping it', (
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
                library: 'lib1',
                libraries: _libraries,
              ),
            ),
          );
        },
      ),
    );

    const String chosen = '${Strings.libraryLabel}: Films';
    expect(
      find.text(chosen),
      findsOneWidget,
      reason: 'the chosen library is named in the control, not just implied',
    );

    await tester.tap(find.text(chosen));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.filterAll).last);
    await tester.pumpAndSettle();

    expect(went, isNot(contains('library=')));
  });
}

void _noop() {}
