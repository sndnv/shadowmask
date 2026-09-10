import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Widget _host(Widget child, {bool disableAnimations = false}) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: MediaQuery(
    data: MediaQueryData(disableAnimations: disableAnimations),
    child: Scaffold(body: child),
  ),
);

Gradient? _gradient(WidgetTester tester) {
  final DecoratedBox box = tester.widget<DecoratedBox>(
    find.descendant(
      of: find.byType(Skeleton).first,
      matching: find.byType(DecoratedBox),
    ),
  );
  return (box.decoration as BoxDecoration).gradient;
}

void main() {
  testWidgets('the shimmer sweeps across the bar', (WidgetTester tester) async {
    await tester.pumpWidget(_host(const SkeletonLines(lines: 1)));

    final Gradient? first = _gradient(tester);
    expect(first, isNotNull);

    await tester.pump(const Duration(milliseconds: 400));
    expect(_gradient(tester), isNot(first));

    await tester.pump(kShimmerDuration);
  });

  testWidgets('reduced motion paints a flat bar with no animation', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(const SkeletonLines(lines: 1), disableAnimations: true),
    );

    expect(_gradient(tester), isNull);

    await tester.pump(const Duration(milliseconds: 400));
    expect(_gradient(tester), isNull);
  });

  testWidgets('a block in flight renders a skeleton, not the word', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        buildBlock<int>(
          future: Future<int>.delayed(const Duration(seconds: 1), () => 1),
          builder: (BuildContext context, int value) => Text('$value'),
        ),
      ),
    );
    await tester.pump(kLoadingRevealDelay);

    expect(find.byType(Skeleton), findsWidgets);
    expect(find.text(Strings.loading), findsNothing);

    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(find.text('1'), findsOneWidget);
    expect(find.byType(Skeleton), findsNothing);
  });

  testWidgets('a caller can pass the block shape', (WidgetTester tester) async {
    await tester.pumpWidget(
      _host(
        buildBlock<int>(
          future: Future<int>.delayed(const Duration(seconds: 1), () => 1),
          loading: const SkeletonRows(rows: 4),
          builder: (BuildContext context, int value) => Text('$value'),
        ),
      ),
    );
    await tester.pump(kLoadingRevealDelay);

    expect(find.byType(SkeletonRows), findsOneWidget);

    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
  });

  testWidgets('a load faster than the reveal delay shows no skeleton', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        buildBlock<int>(
          future: Future<int>.delayed(
            const Duration(milliseconds: 60),
            () => 1,
          ),
          builder: (BuildContext context, int value) => Text('$value'),
        ),
      ),
    );
    await tester.pump(const Duration(milliseconds: 30));
    expect(find.byType(Skeleton), findsNothing);

    await tester.pump(const Duration(milliseconds: 40));
    await tester.pumpAndSettle();
    expect(find.text('1'), findsOneWidget);
    expect(find.byType(Skeleton), findsNothing);
  });

  testWidgets('a page declares one shape for both of its load phases', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        LoadingShape(
          shape: const SkeletonRows(rows: 4),
          child: buildBlock<int>(
            future: Future<int>.delayed(const Duration(seconds: 1), () => 1),
            builder: (BuildContext context, int value) => Text('$value'),
          ),
        ),
      ),
    );
    await tester.pump(kLoadingRevealDelay);

    expect(find.byType(SkeletonRows), findsOneWidget);
    expect(find.byType(SkeletonLines), findsNothing);

    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
  });

  testWidgets('one ticker drives every bar in a shape', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(const SkeletonCards(count: 4)));

    expect(find.byType(SkeletonPulse), findsOneWidget);
    expect(find.byType(Skeleton), findsNWidgets(12));

    await tester.pump(kShimmerDuration);
  });

  testWidgets('the loading toolbar folds onto a phone instead of overflowing', (
    WidgetTester tester,
  ) async {
    // Three fixed 120px chips in a Row came to 408px, which is wider than any
    // phone, and it showed for exactly as long as the page took to load.
    tester.view.physicalSize = const Size(360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      _host(const SkeletonPage(toolbar: true, child: SizedBox.shrink())),
    );

    expect(tester.takeException(), isNull);

    await tester.pump(kShimmerDuration);
  });
}
