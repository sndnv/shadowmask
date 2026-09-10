import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/entry_row.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Future<void> _pump(WidgetTester tester, double width) async {
  tester.view.physicalSize = Size(width, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: SizedBox(
          width: width,
          child: EntryRow(
            label: const Text('Lounge tablet · android', key: Key('label')),
            meta: const Text('Last seen 2 hours ago'),
            action: TextButton(onPressed: () {}, child: const Text('Revoke')),
          ),
        ),
      ),
    ),
  );
  await tester.pump();
}

void main() {
  testWidgets('a phone gives the label the whole width instead of a sliver', (
    WidgetTester tester,
  ) async {
    // On one line the meta text and the button take their intrinsic width
    // first, which squeezed the label to almost nothing and wrapped the
    // device name one character per line.
    await _pump(tester, 360);

    final double label = tester.getSize(find.byKey(const Key('label'))).width;

    expect(label, greaterThan(200));
  });

  testWidgets('a wide viewport keeps all three on one line', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 1200);

    final Rect label = tester.getRect(find.byKey(const Key('label')));
    final Rect action = tester.getRect(find.byType(TextButton));

    expect(label.center.dy, closeTo(action.center.dy, 1));
    expect(label.right, lessThanOrEqualTo(action.left));
  });

  testWidgets('an entry with nothing to say still puts its action right', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: SizedBox(
            width: 360,
            child: EntryRow(
              label: const Text('Lounge tablet · android', key: Key('label')),
              action: TextButton(onPressed: () {}, child: const Text('Revoke')),
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    final Rect action = tester.getRect(find.byType(TextButton));

    expect(action.right, closeTo(360, 1));
  });
}
