import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/default/lazy_block.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Widget _host(Widget child) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(body: child),
);

Widget _below(Widget block, {ScrollController? controller}) => _host(
  SingleChildScrollView(
    controller: controller,
    child: Column(
      children: <Widget>[
        const SizedBox(height: 2000),
        block,
        const SizedBox(height: 2000),
      ],
    ),
  ),
);

void main() {
  testWidgets('an unseen block never calls the api', (
    WidgetTester tester,
  ) async {
    int loads = 0;
    await tester.pumpWidget(
      _below(
        LazyBlock<int>(
          margin: 0,
          load: () async {
            loads++;
            return 1;
          },
          placeholder: const SizedBox(height: 100),
          builder: (BuildContext c, int v) => Text('$v'),
        ),
      ),
    );
    await tester.pump();

    expect(loads, 0);
    expect(find.text('1'), findsNothing);
  });

  testWidgets('scrolling to it loads once and renders the data', (
    WidgetTester tester,
  ) async {
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    int loads = 0;

    await tester.pumpWidget(
      _below(
        LazyBlock<int>(
          margin: 0,
          load: () async {
            loads++;
            return 1;
          },
          placeholder: const SizedBox(height: 100),
          builder: (BuildContext c, int v) => Text('$v'),
        ),
        controller: controller,
      ),
    );
    await tester.pump();

    controller.jumpTo(1900);
    await tester.pumpAndSettle();

    expect(loads, 1);
    expect(find.text('1'), findsOneWidget);
  });

  testWidgets('a failure offers a retry that calls the api again', (
    WidgetTester tester,
  ) async {
    int loads = 0;
    await tester.pumpWidget(
      _host(
        LazyBlock<int>(
          load: () async {
            loads++;
            if (loads == 1) {
              throw Exception('nope');
            }
            return 7;
          },
          builder: (BuildContext c, int v) => Text('$v'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(loads, 1);
    expect(find.text(Strings.couldNotLoad), findsOneWidget);

    await tester.tap(find.text(Strings.retry));
    await tester.pumpAndSettle();

    expect(loads, 2);
    expect(find.text('7'), findsOneWidget);
  });

  testWidgets('a slow load shows the loading shape', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        LazyBlock<int>(
          load: () => Future<int>.delayed(const Duration(seconds: 1), () => 1),
          loading: const SkeletonRows(rows: 3),
          builder: (BuildContext c, int v) => Text('$v'),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(kLoadingRevealDelay);

    expect(find.byType(SkeletonRows), findsOneWidget);

    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();

    expect(find.text('1'), findsOneWidget);
  });

  testWidgets('the idle placeholder falls back to the loading shape', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _below(
        LazyBlock<int>(
          margin: 0,
          load: () async => 1,
          loading: const SkeletonRows(rows: 3),
          builder: (BuildContext c, int v) => Text('$v'),
        ),
      ),
    );
    await tester.pump();

    expect(find.byType(SkeletonRows), findsOneWidget);
  });
}
