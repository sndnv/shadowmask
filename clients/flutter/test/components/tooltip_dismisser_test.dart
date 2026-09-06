import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/tooltip_dismisser.dart';

Future<void> _hover(WidgetTester tester, Finder target) async {
  final TestGesture pointer = await tester.createGesture(
    kind: PointerDeviceKind.mouse,
  );
  await pointer.addPointer(location: tester.getCenter(target));
  addTearDown(pointer.removePointer);
  await tester.pump(const Duration(seconds: 2));
  await tester.pumpAndSettle();
}

Widget _app() => const MaterialApp(
  home: TooltipDismisser(
    child: Scaffold(
      body: Center(
        child: Tooltip(
          message: 'Arrival',
          child: SizedBox(width: 120, height: 120),
        ),
      ),
    ),
  ),
);

void main() {
  testWidgets('resizing the view puts an open tooltip away', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1036, 981);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(_app());
    await _hover(tester, find.byType(Tooltip));
    expect(find.text('Arrival'), findsOneWidget);

    tester.view.physicalSize = const Size(1512, 982);
    await tester.pumpAndSettle();

    expect(
      find.text('Arrival'),
      findsNothing,
      reason:
          'a tooltip left up across a resize lays its overlay child out against '
          'the old view size, which trips a framework assertion and leaves the '
          'render tree uploading zero sized textures',
    );
  });

  testWidgets('a tooltip that was never shown survives a resize', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1036, 981);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    tester.view.physicalSize = const Size(1512, 982);
    await tester.pumpAndSettle();

    expect(find.byType(Tooltip), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('the dismisser passes its child straight through', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(home: TooltipDismisser(child: Text('body'))),
    );
    expect(find.text('body'), findsOneWidget);
  });
}
