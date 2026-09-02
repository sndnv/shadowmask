import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/admin_header_split.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Widget _host(double width) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(
    body: SizedBox(
      width: width,
      child: const AdminHeaderSplit(
        start: SizedBox(key: Key('start'), height: 40),
        end: SizedBox(key: Key('end'), height: 40),
      ),
    ),
  ),
);

void main() {
  testWidgets('a wide split shares one row', (WidgetTester tester) async {
    await tester.pumpWidget(_host(760));

    final Rect start = tester.getRect(find.byKey(const Key('start')));
    final Rect end = tester.getRect(find.byKey(const Key('end')));

    expect(end.top, start.top);
    expect(end.left, greaterThan(start.right));
    expect(start.width, end.width);
  });

  testWidgets('a narrow split stacks and each half takes the row', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(304));

    final Rect start = tester.getRect(find.byKey(const Key('start')));
    final Rect end = tester.getRect(find.byKey(const Key('end')));

    // Two halves of a 304px body are 144px each, which fits neither.
    expect(end.top, greaterThan(start.bottom - 1));
    expect(start.left, end.left);
    expect(start.width, 304);
    expect(end.width, 304);
  });

  testWidgets('a fixed start keeps its width and gets the rest beside it', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_fixedHost(480));

    final Rect start = tester.getRect(find.byKey(const Key('start')));
    final Rect end = tester.getRect(find.byKey(const Key('end')));

    expect(start.width, 96);
    expect(end.left, greaterThan(start.right - 1));
    expect(end.top, start.top);
  });

  testWidgets('a fixed start does not stretch when the pair stacks', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_fixedHost(300));

    final Rect start = tester.getRect(find.byKey(const Key('start')));
    final Rect end = tester.getRect(find.byKey(const Key('end')));

    // A 96px poster must stay a poster, not grow to fill the row.
    expect(start.width, 96);
    expect(end.top, greaterThan(start.bottom - 1));
    expect(end.left, start.left);
  });
}

Widget _fixedHost(double width) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(
    body: SizedBox(
      width: width,
      child: const AdminHeaderSplit(
        startWidth: 96,
        breakpoint: 392,
        start: SizedBox(key: Key('start'), height: 140),
        end: SizedBox(key: Key('end'), height: 40),
      ),
    ),
  ),
);
