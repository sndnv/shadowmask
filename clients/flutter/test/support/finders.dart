import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/field_help.dart';

Future<void> hover(WidgetTester tester, Finder target) async {
  final TestGesture pointer = await tester.createGesture(
    kind: PointerDeviceKind.mouse,
  );
  await pointer.addPointer(location: Offset.zero);
  addTearDown(pointer.removePointer);
  await tester.pump();
  await pointer.moveTo(tester.getCenter(target));
  await tester.pumpAndSettle();
}

Finder fieldNamed(String label) => find.descendant(
  of: find.ancestor(of: find.text(label), matching: find.byType(FieldLabel)),
  matching: find.byType(TextField),
);

Color? rowTintOf(WidgetTester tester, String cellText) {
  // A focus ring is a DecoratedBox as well, and on a cell that is itself a
  // link it sits between the text and the row that paints the tint.
  for (final DecoratedBox box in tester.widgetList<DecoratedBox>(
    find.ancestor(of: find.text(cellText), matching: find.byType(DecoratedBox)),
  )) {
    final BoxDecoration decoration = box.decoration as BoxDecoration;
    if (decoration.color == null && (decoration.border?.isUniform ?? false)) {
      continue;
    }
    return decoration.color;
  }
  return null;
}
