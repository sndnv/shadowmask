import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/load_more.dart';

Widget _scrolled({
  required VoidCallback onLoad,
  required ScrollController controller,
  bool enabled = true,
  double bodyHeight = 3000,
  double threshold = kLoadMoreThreshold,
}) => MaterialApp(
  home: Scaffold(
    body: SingleChildScrollView(
      controller: controller,
      child: Column(
        children: <Widget>[
          SizedBox(height: bodyHeight),
          LoadMore(
            onLoad: onLoad,
            enabled: enabled,
            threshold: threshold,
            child: const SizedBox(height: 20),
          ),
        ],
      ),
    ),
  ),
);

void main() {
  testWidgets('stays quiet while the end is far away', (
    WidgetTester tester,
  ) async {
    int loads = 0;
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      _scrolled(onLoad: () => loads++, controller: controller),
    );
    await tester.pumpAndSettle();

    expect(loads, 0);
  });

  testWidgets('fires once the end comes within the threshold', (
    WidgetTester tester,
  ) async {
    int loads = 0;
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      _scrolled(onLoad: () => loads++, controller: controller),
    );
    await tester.pumpAndSettle();

    controller.jumpTo(controller.position.maxScrollExtent - 100);
    await tester.pumpAndSettle();

    expect(loads, greaterThan(0));
  });

  testWidgets('fires straight away when the content does not fill the view', (
    WidgetTester tester,
  ) async {
    int loads = 0;
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      _scrolled(onLoad: () => loads++, controller: controller, bodyHeight: 10),
    );
    await tester.pumpAndSettle();

    expect(loads, greaterThan(0));
  });

  testWidgets('disabled never fires, however far it is scrolled', (
    WidgetTester tester,
  ) async {
    int loads = 0;
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      _scrolled(onLoad: () => loads++, controller: controller, enabled: false),
    );
    await tester.pumpAndSettle();

    controller.jumpTo(controller.position.maxScrollExtent);
    await tester.pumpAndSettle();

    expect(loads, 0);
  });

  testWidgets('a resize re-checks, even when nothing can scroll any more', (
    WidgetTester tester,
  ) async {
    int loads = 0;
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    tester.view.physicalSize = const Size(600, 400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      _scrolled(onLoad: () => loads++, controller: controller, bodyHeight: 900),
    );
    await tester.pumpAndSettle();

    expect(loads, 0);

    tester.view.physicalSize = const Size(600, 2000);
    await tester.pumpAndSettle();

    expect(loads, greaterThan(0));
  });

  testWidgets('a deeper threshold fires earlier, from further up the list', (
    WidgetTester tester,
  ) async {
    int shallow = 0;
    int deep = 0;
    final ScrollController a = ScrollController();
    final ScrollController b = ScrollController();
    addTearDown(a.dispose);
    addTearDown(b.dispose);

    await tester.pumpWidget(
      _scrolled(onLoad: () => shallow++, controller: a, threshold: 100),
    );
    await tester.pumpAndSettle();
    a.jumpTo(a.position.maxScrollExtent - 500);
    await tester.pumpAndSettle();

    await tester.pumpWidget(
      _scrolled(onLoad: () => deep++, controller: b, threshold: 900),
    );
    await tester.pumpAndSettle();
    b.jumpTo(b.position.maxScrollExtent - 500);
    await tester.pumpAndSettle();

    expect(shallow, 0);
    expect(deep, greaterThan(0));
  });

  testWidgets('no enclosing scrollable never fires', (
    WidgetTester tester,
  ) async {
    int loads = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: LoadMore(
            onLoad: () => loads++,
            child: const SizedBox(height: 20),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(loads, 0);
  });
}
