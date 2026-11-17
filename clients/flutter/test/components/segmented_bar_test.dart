import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/segmented_bar.dart';

const Color _track = Color(0xFF102030);
const Color _fill = Color(0xFF45B8FF);

Future<void> _pump(WidgetTester tester, double fraction) => tester.pumpWidget(
  Directionality(
    textDirection: TextDirection.ltr,
    child: Center(
      child: SizedBox(
        width: 32,
        child: SegmentedBar(fraction: fraction, track: _track, fill: _fill),
      ),
    ),
  ),
);

Finder get _bar => find.descendant(
  of: find.byType(SegmentedBar),
  matching: find.byType(CustomPaint),
);

PaintPattern _segments(PaintPattern pattern, Color color) {
  for (final double left in const <double>[0, 8, 16, 24]) {
    pattern.rect(rect: Rect.fromLTWH(left, 0, 6, 6), color: color);
  }
  return pattern;
}

void main() {
  testWidgets('the bar is six tall across its width', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 0.5);

    expect(tester.getSize(find.byType(SegmentedBar)), const Size(32, 6));
  });

  testWidgets('segments fill to the percent and the rest is track', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 0.5);

    final PaintPattern pattern = paints
      ..save()
      ..clipRect(rect: const Rect.fromLTRB(0, 0, 16, 6));
    _segments(pattern, _fill)
      ..restore()
      ..save()
      ..clipRect(rect: const Rect.fromLTRB(16, 0, 32, 6));
    _segments(pattern, _track).restore();

    expect(_bar, pattern);
  });

  testWidgets('a percent past either end stays inside the bar', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 1.4);
    expect(_bar, paints..clipRect(rect: const Rect.fromLTRB(0, 0, 32, 6)));

    await _pump(tester, -0.2);
    expect(_bar, paints..clipRect(rect: const Rect.fromLTRB(0, 0, 0, 6)));
  });
}
