import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/pagination.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Future<void> _pump(
  WidgetTester tester, {
  double width = 800,
  int total = 120,
  int offset = 20,
  int count = 20,
}) async {
  tester.view.physicalSize = Size(width, 600);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: Column(
          // Every caller puts the pager in a start-aligned column, which is
          // what left it hanging off the edge of the page.
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Pagination(
              basePath: '/admin/users',
              params: const <String, String?>{},
              total: total,
              offset: offset,
              limit: 20,
              count: count,
            ),
          ],
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('the pager sits in the middle of the page', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    final Rect prev = tester.getRect(
      find.widgetWithText(OutlinedButton, Strings.previous),
    );
    final Rect next = tester.getRect(
      find.widgetWithText(OutlinedButton, Strings.next),
    );

    expect((prev.left + next.right) / 2, closeTo(400, 1));
  });

  testWidgets('a phone keeps it centred and wraps instead of overflowing', (
    WidgetTester tester,
  ) async {
    await _pump(tester, width: 360);

    final Rect pager = tester.getRect(find.byType(Wrap));
    final Rect next = tester.getRect(
      find.widgetWithText(OutlinedButton, Strings.next),
    );

    expect(pager.center.dx, closeTo(180, 1));
    expect(pager.width, lessThanOrEqualTo(360));
    expect(
      next.top,
      greaterThan(pager.top),
      reason: 'too tight for one row, so it wraps rather than overflowing',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('nothing is drawn when there is only one page', (
    WidgetTester tester,
  ) async {
    await _pump(tester, total: 0, offset: 0, count: 0);

    expect(find.byType(OutlinedButton), findsNothing);
  });
}
