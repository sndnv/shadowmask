import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/title_heading.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Future<void> _pump(
  WidgetTester tester,
  TitleHeading heading, {
  double? width,
}) async {
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: SizedBox(width: width, child: heading),
        ),
      ),
    ),
  );
  await tester.pump();
}

TitleAction _action(IconData icon, {VoidCallback? onPressed}) =>
    TitleAction(icon: icon, tooltip: 'x', onPressed: onPressed);

void main() {
  testWidgets('a divider separates the pager from the other actions', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      TitleHeading(
        title: 'Episode 1',
        pager: <TitleAction>[
          _action(Icons.chevron_left, onPressed: () {}),
          _action(Icons.chevron_right, onPressed: () {}),
        ],
        actions: <TitleAction>[_action(Icons.edit, onPressed: () {})],
      ),
    );

    expect(find.text('|'), findsOneWidget);
  });

  testWidgets('a pager on its own needs no divider', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      TitleHeading(
        title: 'Season 2',
        pager: <TitleAction>[_action(Icons.chevron_left, onPressed: () {})],
      ),
    );

    expect(find.byIcon(Icons.chevron_left), findsOneWidget);
    expect(find.text('|'), findsNothing);
  });

  testWidgets('actions on their own keep the old layout', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      TitleHeading(
        title: 'Blade Runner',
        actions: <TitleAction>[_action(Icons.edit, onPressed: () {})],
      ),
    );

    expect(find.text('|'), findsNothing);
  });

  testWidgets('an action with no callback renders disabled, not missing', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      TitleHeading(
        title: 'Episode 1',
        pager: <TitleAction>[_action(Icons.chevron_left)],
      ),
    );

    expect(find.byIcon(Icons.chevron_left), findsOneWidget);
    final IconButton button = tester.widget<IconButton>(
      find.byType(IconButton),
    );
    expect(
      button.onPressed,
      isNull,
      reason: 'the edge of a series must not make the control jump around',
    );
  });

  testWidgets('a wide heading keeps its actions on the title row', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      TitleHeading(
        title: 'Blade Runner',
        actions: <TitleAction>[
          _action(Icons.edit, onPressed: () {}),
          _action(Icons.link, onPressed: () {}),
        ],
      ),
      width: 700,
    );

    final Rect title = tester.getRect(find.text('Blade Runner'));
    final Rect edit = tester.getRect(find.byIcon(Icons.edit));

    expect(edit.left, greaterThan(title.right - 1));
    expect(edit.center.dy, closeTo(title.center.dy, 12));
  });

  testWidgets('a narrow heading drops its actions to their own row', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      TitleHeading(
        title: 'A rather long film title that wants the room',
        actions: <TitleAction>[
          _action(Icons.edit, onPressed: () {}),
          _action(Icons.link, onPressed: () {}),
          _action(Icons.refresh, onPressed: () {}),
        ],
      ),
      width: 304,
    );

    expect(tester.takeException(), isNull);
    final Rect title = tester.getRect(
      find.text('A rather long film title that wants the room'),
    );
    final Rect edit = tester.getRect(find.byIcon(Icons.edit));

    // The buttons stop competing with the title for the same row.
    expect(edit.top, greaterThan(title.bottom - 1));
    expect(edit.left, lessThan(title.left + 40));
  });

  testWidgets('a lone action follows the title instead of taking a row', (
    WidgetTester tester,
  ) async {
    // A title wraps rather than ellipsing, so the last line almost always has
    // room for one icon, and a row of its own left 90% of it empty.
    await _pump(
      tester,
      TitleHeading(
        title: 'A rather long film title that wants the room',
        actions: <TitleAction>[_action(Icons.layers, onPressed: () {})],
      ),
      width: 304,
    );

    expect(find.byType(Wrap), findsNothing);
    expect(
      tester.getRect(find.byIcon(Icons.layers)).top,
      lessThan(tester.getRect(find.byType(Text).first).bottom),
      reason: 'inside the text, not below it',
    );
  });

  testWidgets('several actions still get their own row', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      TitleHeading(
        title: 'A rather long film title that wants the room',
        actions: <TitleAction>[
          _action(Icons.layers, onPressed: () {}),
          _action(Icons.edit, onPressed: () {}),
        ],
      ),
      width: 304,
    );

    expect(find.byType(Wrap), findsOneWidget);
  });

  testWidgets('a pager keeps its own row even with one action', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      TitleHeading(
        title: 'A rather long film title that wants the room',
        pager: <TitleAction>[_action(Icons.chevron_left, onPressed: () {})],
        actions: <TitleAction>[_action(Icons.layers, onPressed: () {})],
      ),
      width: 304,
    );

    expect(find.byType(Wrap), findsOneWidget);
  });
}
