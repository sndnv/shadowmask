import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/empty_note.dart';
import 'package:shadowmask/components/link_chip.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/empty_state.dart';

Future<void> _pump(WidgetTester tester, EmptyState state) async {
  tester.view.physicalSize = const Size(800, 600);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: Column(
          // Every caller puts it in a start-aligned column, which is what
          // left it hugging the left edge.
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[EmptyNote(state)],
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('the line sits in the middle of the page', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const EmptyState('Nothing to show yet.'));

    expect(
      tester.getRect(find.text('Nothing to show yet.')).center.dx,
      closeTo(400, 1),
    );
  });

  testWidgets('an offered action is centred under it', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      const EmptyState(
        'No libraries yet.',
        actionLabel: 'Create a library',
        actionRoute: '/admin/libraries',
      ),
    );

    final Rect text = tester.getRect(find.text('No libraries yet.'));
    final Rect action = tester.getRect(find.byType(LinkChip));

    expect(action.center.dx, closeTo(400, 1));
    expect(action.top, greaterThan(text.bottom));
  });

  testWidgets('a long line wraps centred rather than ragged', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      const EmptyState(
        'Search across movies, series, episodes and people, and anything '
        'else this library happens to hold onto for you.',
      ),
    );

    expect(tester.widget<Text>(find.byType(Text)).textAlign, TextAlign.center);
    expect(tester.takeException(), isNull);
  });
}
