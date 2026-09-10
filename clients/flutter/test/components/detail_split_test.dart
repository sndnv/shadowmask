import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/detail_split.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Future<void> _pump(
  WidgetTester tester,
  double width, {
  Widget? headline,
  Widget? actions,
  double? compactPosterWidth,
}) async {
  tester.view.physicalSize = Size(width, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: DetailSplit(
            posterWidth: 320,
            compactPosterWidth: compactPosterWidth,
            headline: headline,
            actions: actions,
            poster: const SizedBox(key: Key('poster'), height: 180),
            info: const Text('info'),
          ),
        ),
      ),
    ),
  );
}

double _top(WidgetTester tester, Finder of) => tester.getRect(of).top;

void main() {
  testWidgets('a wide split keeps the poster at its asked width', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 1200);

    expect(
      tester.getSize(find.byKey(const Key('poster'))).width,
      closeTo(320, 2),
      reason: 'the frame spends a pixel of border on each side',
    );
  });

  testWidgets('a phone shrinks the poster to the width it actually has', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 300);

    expect(tester.takeException(), isNull);
    expect(
      tester.getSize(find.byKey(const Key('poster'))).width,
      lessThanOrEqualTo(300),
      reason: 'collapsing to one column does not pay for a 320px poster',
    );
  });

  testWidgets('a phone puts the actions above the rest of the detail', (
    WidgetTester tester,
  ) async {
    // Play sat after the overview, which put it off the bottom of a phone.
    await _pump(
      tester,
      360,
      headline: const Text('headline'),
      actions: const Text('actions'),
    );

    final double poster = _top(tester, find.byKey(const Key('poster')));
    final double headline = _top(tester, find.text('headline'));
    final double actions = _top(tester, find.text('actions'));
    final double info = _top(tester, find.text('info'));

    expect(poster, lessThan(headline));
    expect(headline, lessThan(actions));
    expect(actions, lessThan(info));
  });

  testWidgets('a wide split leaves the actions where they were, at the end', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      1200,
      headline: const Text('headline'),
      actions: const Text('actions'),
    );

    expect(
      _top(tester, find.text('info')),
      lessThan(_top(tester, find.text('actions'))),
      reason: 'the desktop order was not what needed fixing',
    );
    expect(
      tester.getRect(find.byKey(const Key('poster'))).right,
      lessThanOrEqualTo(tester.getRect(find.text('headline')).left),
      reason: 'poster beside the text, not above it',
    );
  });

  testWidgets('a page can ask for a full width poster on a phone', (
    WidgetTester tester,
  ) async {
    // An episode still is landscape, so cropping it to a poster width wastes
    // the screen it was shot for.
    await _pump(
      tester,
      360,
      headline: const Text('headline'),
      compactPosterWidth: double.infinity,
    );

    expect(
      tester.getSize(find.byKey(const Key('poster'))).width,
      closeTo(360, 2),
    );
  });

  testWidgets('a page that asks for nothing new keeps the old single column', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 360);

    expect(
      _top(tester, find.byKey(const Key('poster'))),
      lessThan(_top(tester, find.text('info'))),
    );
  });
}
