import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

import '../../support/finders.dart';

Future<void> _pump(WidgetTester tester, Widget child) async {
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(body: SizedBox(width: 1000, child: child)),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

List<AdminColumn<String>> _columns() => <AdminColumn<String>>[
  AdminColumn<String>(
    label: 'Name',
    sortKey: (String s) => s,
    cell: (BuildContext c, String s) => Text(s),
  ),
];

void main() {
  testWidgets('an end-aligned column right-aligns its header and its cells', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTable<String>(
        columns: <AdminColumn<String>>[
          AdminColumn<String>(
            label: 'Name',
            cell: (BuildContext c, String s) => Text(s),
          ),
          AdminColumn<String>(
            label: 'Actions',
            align: AdminColumnAlign.end,
            cell: (BuildContext c, String s) => Text('do $s'),
          ),
        ],
        rows: const <String>['apple'],
        emptyText: 'None',
      ),
    );

    final double cellRight = tester.getTopRight(find.text('do apple')).dx;
    final double headerRight = tester.getTopRight(find.text('ACTIONS')).dx;
    final double nameRight = tester.getTopRight(find.text('apple')).dx;

    expect(cellRight, greaterThan(nameRight));
    expect(headerRight, closeTo(cellRight, 1));
  });

  testWidgets('renders a cell per row', (WidgetTester tester) async {
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['banana', 'apple'],
        emptyText: 'None',
      ),
    );

    expect(find.text('banana'), findsOneWidget);
    expect(find.text('apple'), findsOneWidget);
    expect(find.text('None'), findsNothing);
  });

  testWidgets('shows the empty text when there are no rows', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>[],
        emptyText: 'None',
      ),
    );

    expect(find.text('None'), findsOneWidget);
  });

  testWidgets('sorts ascending when a sortable header is tapped', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['banana', 'apple'],
        emptyText: 'None',
      ),
    );

    await tester.tap(find.text('NAME'));
    await tester.pumpAndSettle();

    final double apple = tester.getTopLeft(find.text('apple')).dy;
    final double banana = tester.getTopLeft(find.text('banana')).dy;
    expect(apple, lessThan(banana));
  });

  testWidgets('paints only the rows rowColor returns a tint for', (
    WidgetTester tester,
  ) async {
    const Color tint = Color(0x22FF0000);
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['tinted', 'plain'],
        emptyText: 'None',
        rowColor: (String s) => s == 'tinted' ? tint : null,
      ),
    );

    expect(rowTintOf(tester, 'tinted'), tint);
    expect(rowTintOf(tester, 'plain'), isNull);
  });

  testWidgets('an initial sort column orders the rows before any tap', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['banana', 'apple'],
        emptyText: 'None',
        initialSortColumn: 0,
      ),
    );

    final double apple = tester.getTopLeft(find.text('apple')).dy;
    final double banana = tester.getTopLeft(find.text('banana')).dy;
    expect(apple, lessThan(banana));
  });

  testWidgets('tapping the initially sorted header reverses it', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['banana', 'apple'],
        emptyText: 'None',
        initialSortColumn: 0,
      ),
    );

    await tester.tap(find.text('NAME'));
    await tester.pumpAndSettle();

    final double apple = tester.getTopLeft(find.text('apple')).dy;
    final double banana = tester.getTopLeft(find.text('banana')).dy;
    expect(banana, lessThan(apple));
  });

  testWidgets('a table that fits its space does not scroll sideways', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['apple'],
        emptyText: 'None',
        minWidth: 400,
      ),
    );

    final ScrollableState scroller = tester.state(find.byType(Scrollable));
    expect(scroller.position.maxScrollExtent, 0);
  });

  testWidgets('a table wider than its space scrolls to its minimum width', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['apple'],
        emptyText: 'None',
        minWidth: 1400,
      ),
    );

    final ScrollableState scroller = tester.state(find.byType(Scrollable));
    expect(scroller.position.maxScrollExtent, greaterThan(390));
  });

  testWidgets('invokes onRowTap with the tapped row', (
    WidgetTester tester,
  ) async {
    String? tapped;
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['a', 'b'],
        emptyText: 'None',
        onRowTap: (String r) => tapped = r,
      ),
    );

    await tester.tap(find.text('a'));
    await tester.pumpAndSettle();

    expect(tapped, 'a');
  });

  testWidgets('hovering a row lifts it off the table surface', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['a', 'b'],
        emptyText: 'None',
      ),
    );

    expect(rowTintOf(tester, 'a'), isNull);

    await hover(tester, find.text('a'));

    expect(
      rowTintOf(tester, 'a'),
      Color.alphaBlend(Tokens.dark.rowHover, Tokens.dark.surface),
    );
    expect(
      rowTintOf(tester, 'b'),
      isNull,
      reason: 'only the row under the pointer changes',
    );
  });

  testWidgets('a hovered row keeps its status tint underneath', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['a'],
        emptyText: 'None',
        rowColor: (String _) => Tokens.dark.rowDanger,
      ),
    );

    expect(rowTintOf(tester, 'a'), Tokens.dark.rowDanger);

    await hover(tester, find.text('a'));

    expect(
      rowTintOf(tester, 'a'),
      Color.alphaBlend(
        Tokens.dark.rowHover,
        Color.alphaBlend(Tokens.dark.rowDanger, Tokens.dark.surface),
      ),
      reason: 'the hover composites over the tint rather than replacing it',
    );
  });

  testWidgets('a row that is not tappable still highlights', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['a'],
        emptyText: 'None',
      ),
    );

    await hover(tester, find.text('a'));

    expect(rowTintOf(tester, 'a'), isNotNull);
    expect(
      find.byType(HoverTap),
      findsNothing,
      reason: 'the click cursor is what promises a tap, not the highlight',
    );
  });

  testWidgets('the keyboard opens a row without losing its own actions', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    final List<String> opened = <String>[];
    int deleted = 0;
    await _pump(
      tester,
      AdminTable<String>(
        columns: <AdminColumn<String>>[
          AdminColumn<String>(
            label: 'Name',
            cell: (BuildContext c, String s) => Text(s),
          ),
          AdminColumn<String>(
            label: 'Actions',
            cell: (BuildContext c, String s) => IconButton(
              tooltip: 'Delete $s',
              icon: const Icon(Icons.delete),
              onPressed: () => deleted++,
            ),
          ),
        ],
        rows: const <String>['ada'],
        emptyText: 'None',
        onRowTap: opened.add,
        rowLabel: 'Open user',
      ),
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pump();

    expect(opened, <String>['ada']);

    // The row reads as one item, and the delete button inside it is still a
    // separate control rather than being folded into the row.
    expect(tester.getSemantics(find.byType(HoverTap)).label, 'Open user\nada');
    expect(find.byTooltip('Delete ada'), findsOneWidget);
    await tester.tap(find.byTooltip('Delete ada'));
    await tester.pump();
    expect(deleted, 1);

    semantics.dispose();
  });

  _priorityColumnTests();
}

Future<void> _pumpAt(WidgetTester tester, double width, Widget child) async {
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: SizedBox(width: width, child: child),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

List<AdminColumn<String>> _threeColumns() => <AdminColumn<String>>[
  AdminColumn<String>(
    label: 'Name',
    essential: true,
    sortKey: (String s) => s,
    cell: (BuildContext c, String s) => Text(s),
  ),
  AdminColumn<String>(
    label: 'Kind',
    essential: true,
    cell: (BuildContext c, String s) => Text('kind-$s'),
  ),
  AdminColumn<String>(
    label: 'Origin',
    sortKey: (String s) => s,
    cell: (BuildContext c, String s) => Text('origin-$s'),
  ),
];

void _priorityColumnTests() {
  testWidgets('a wide table shows every column', (WidgetTester tester) async {
    await _pumpAt(
      tester,
      900,
      AdminTable<String>(
        columns: _threeColumns(),
        rows: const <String>['a'],
        emptyText: 'None',
      ),
    );

    expect(find.text('ORIGIN'), findsOneWidget);
    expect(find.text('origin-a'), findsOneWidget);
  });

  testWidgets('a phone table keeps only the essential columns', (
    WidgetTester tester,
  ) async {
    await _pumpAt(
      tester,
      320,
      AdminTable<String>(
        columns: _threeColumns(),
        rows: const <String>['a'],
        emptyText: 'None',
      ),
    );

    expect(tester.takeException(), isNull);
    expect(find.text('NAME'), findsOneWidget);
    expect(find.text('KIND'), findsOneWidget);
    expect(find.text('ORIGIN'), findsNothing);
    expect(find.text('origin-a'), findsNothing);
  });

  testWidgets('a phone table drops the minimum width so nothing scrolls', (
    WidgetTester tester,
  ) async {
    await _pumpAt(
      tester,
      320,
      AdminTable<String>(
        columns: _threeColumns(),
        rows: const <String>['a'],
        emptyText: 'None',
      ),
    );

    final ScrollableState scrollable = tester.state<ScrollableState>(
      find.byType(Scrollable).first,
    );
    expect(scrollable.position.maxScrollExtent, 0);
  });

  testWidgets('a hidden sort column hands sorting to a visible one', (
    WidgetTester tester,
  ) async {
    await _pumpAt(
      tester,
      320,
      AdminTable<String>(
        columns: _threeColumns(),
        rows: const <String>['b', 'a'],
        emptyText: 'None',
        initialSortColumn: 2,
      ),
    );

    // Column 2 is Origin, which a phone hides. Sorting must not stay pinned to
    // a column the reader can neither see nor change.
    expect(find.byIcon(Icons.arrow_upward), findsOneWidget);
    final Rect name = tester.getRect(find.text('NAME'));
    final Rect arrow = tester.getRect(find.byIcon(Icons.arrow_upward));
    expect(arrow.left, greaterThan(name.left));
    expect(
      tester.getRect(find.text('a')).top,
      lessThan(tester.getRect(find.text('b')).top),
    );
  });

  testWidgets('a table with no essential column keeps all of them', (
    WidgetTester tester,
  ) async {
    await _pumpAt(
      tester,
      320,
      AdminTable<String>(
        columns: _columns(),
        rows: const <String>['a'],
        emptyText: 'None',
      ),
    );

    expect(find.text('NAME'), findsOneWidget);
  });

  testWidgets('a zero width parent does not drive the table negative', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: Scaffold(
            body: SizedBox(
              width: 0,
              child: AdminTable<String>(
                columns: _columns(),
                rows: const <String>['a'],
                emptyText: 'None',
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      tester.takeException(),
      isNull,
      reason:
          'subtracting the border from a zero width used to reach -2.0, '
          'which is not a legal BoxConstraints minimum',
    );
  });
}
