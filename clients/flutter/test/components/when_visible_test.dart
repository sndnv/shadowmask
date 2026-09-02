import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/when_visible.dart';

Widget _host(Widget child) => MaterialApp(home: Scaffold(body: child));

Widget _scrolled({
  required ScrollController controller,
  required VoidCallback onVisible,
  double above = 2000,
  double margin = 0,
}) => _host(
  SingleChildScrollView(
    controller: controller,
    child: Column(
      children: <Widget>[
        SizedBox(height: above),
        WhenVisible(
          onVisible: onVisible,
          margin: margin,
          child: const SizedBox(height: 100, child: Text('rail')),
        ),
        const SizedBox(height: 2000),
      ],
    ),
  ),
);

void main() {
  testWidgets('with no scrollable above it, it fires immediately', (
    WidgetTester tester,
  ) async {
    int calls = 0;
    await tester.pumpWidget(
      _host(WhenVisible(onVisible: () => calls++, child: const Text('rail'))),
    );
    await tester.pump();

    expect(calls, 1);
  });

  testWidgets('below the fold it stays quiet until scrolled to', (
    WidgetTester tester,
  ) async {
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    int calls = 0;

    await tester.pumpWidget(
      _scrolled(controller: controller, onVisible: () => calls++),
    );
    await tester.pump();
    expect(calls, 0);

    controller.jumpTo(1900);
    await tester.pump();

    expect(calls, 1);
  });

  testWidgets('it fires once and never again', (WidgetTester tester) async {
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    int calls = 0;

    await tester.pumpWidget(
      _scrolled(controller: controller, onVisible: () => calls++),
    );
    await tester.pump();

    controller.jumpTo(1900);
    await tester.pump();
    expect(calls, 1);

    controller.jumpTo(0);
    await tester.pump();
    controller.jumpTo(1900);
    await tester.pump();

    expect(calls, 1);
  });

  testWidgets('the margin fires it before it is strictly on screen', (
    WidgetTester tester,
  ) async {
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    int eager = 0;

    await tester.pumpWidget(
      _scrolled(controller: controller, onVisible: () => eager++, margin: 400),
    );
    await tester.pump();
    expect(eager, 0);

    controller.jumpTo(1500);
    await tester.pump();

    expect(eager, 1);
  });

  testWidgets('it is already visible when it starts on screen', (
    WidgetTester tester,
  ) async {
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    int calls = 0;

    await tester.pumpWidget(
      _scrolled(controller: controller, onVisible: () => calls++, above: 0),
    );
    await tester.pump();

    expect(calls, 1);
  });

  testWidgets('disposing while still unseen removes its listener', (
    WidgetTester tester,
  ) async {
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    int calls = 0;

    await tester.pumpWidget(
      _scrolled(controller: controller, onVisible: () => calls++),
    );
    await tester.pump();

    await tester.pumpWidget(_host(const Text('gone')));
    await tester.pump();

    expect(calls, 0);
    expect(tester.takeException(), isNull);
  });
}
