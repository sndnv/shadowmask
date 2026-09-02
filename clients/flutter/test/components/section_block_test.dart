import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Future<void> _pump(WidgetTester tester, Size size) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: SectionBlock(
            title: 'Versions',
            actions: <Widget>[
              OutlinedButton(onPressed: () {}, child: const Text('Rescan')),
              OutlinedButton(onPressed: () {}, child: const Text('Refresh')),
              OutlinedButton(onPressed: () {}, child: const Text('Delete')),
            ],
            child: const Text('body'),
          ),
        ),
      ),
    ),
  );
}

void main() {
  testWidgets('a wide block keeps the title and actions on one line', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const Size(1200, 800));

    expect(
      tester.getRect(find.text('Delete')).top,
      closeTo(tester.getRect(find.text('Versions')).top, 12),
    );
  });

  testWidgets('the actions are pinned to the right of their section', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const Size(1200, 800));

    final Rect block = tester.getRect(find.byType(SectionBlock));
    final Rect first = tester.getRect(
      find.widgetWithText(OutlinedButton, 'Rescan'),
    );
    final Rect last = tester.getRect(
      find.widgetWithText(OutlinedButton, 'Delete'),
    );
    final Rect title = tester.getRect(find.text('Versions'));

    expect(
      last.right,
      moreOrLessEquals(block.right, epsilon: 1),
      reason:
          'a Wrap that shrink-wraps has no free space to spread, which '
          'left the actions sitting against the title',
    );
    expect(first.left, greaterThan(title.right + 100));
  });

  testWidgets('a phone drops the actions below the title', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const Size(360, 800));

    expect(tester.takeException(), isNull);
    expect(
      tester.getRect(find.text('Rescan')).top,
      greaterThan(tester.getRect(find.text('Versions')).bottom),
      reason: 'three buttons beside a heading need more than 360px',
    );
  });

  testWidgets('a title too long for the row is ellipsised, not overflowed', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: const Scaffold(
          body: SectionBlock(
            title: '0f8b2c1e-7a44-4d19-9c3f-2b6e8a05d731',
            child: Text('body'),
          ),
        ),
      ),
    );

    expect(tester.takeException(), isNull);
    final Text title = tester.widget<Text>(
      find.text('0f8b2c1e-7a44-4d19-9c3f-2b6e8a05d731'),
    );
    // A job id is a section title, and it must not paint past its box.
    expect(title.maxLines, 1);
    expect(title.overflow, TextOverflow.ellipsis);
    expect(
      tester.getSize(find.byType(Text).first).width,
      lessThanOrEqualTo(360),
    );
  });
}
