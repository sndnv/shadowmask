import 'dart:math' as math;

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadowmask/theme/shapes.dart';

import '../support/raster.dart';

const Key _box = ValueKey<String>('box');
const Color _ink = Color(0xFFFFFFFF);
const Color _ground = Color(0xFF000000);
const Rect _rect = Rect.fromLTWH(0, 0, 60, 40);

BorderRadius _bevel(double cut) => BorderRadius.only(
  topLeft: Radius.circular(cut),
  bottomRight: Radius.circular(cut),
);

Offset _topLeftCut(double inward) {
  final double step = inward / math.sqrt2;
  return Offset(5 + step, 5 + step);
}

Offset _bottomRightCut(double inward) {
  final double step = inward / math.sqrt2;
  return Offset(55 - step, 35 - step);
}

Offset _along(Offset from, Offset to, double inward) {
  final Offset run = to - from;
  final Offset normal = Offset(-run.dy, run.dx) / run.distance;
  return (from + to) / 2 + normal * inward;
}

Future<void> _pump(
  WidgetTester tester,
  BorderSide side, {
  BorderRadius? radius,
}) => tester.pumpWidget(
  Directionality(
    textDirection: TextDirection.ltr,
    child: Center(
      child: SizedBox(
        width: _rect.width,
        height: _rect.height,
        child: DecoratedBox(
          key: _box,
          decoration: ShapeDecoration(
            shape: CutBorder(side: side, borderRadius: radius ?? _bevel(10)),
          ),
        ),
      ),
    ),
  ),
);

void main() {
  group('painting', () {
    testWidgets('fills a band as wide as its side inside the box', (
      WidgetTester tester,
    ) async {
      await _pump(tester, const BorderSide(color: _ink));

      expect(
        find.byKey(_box),
        paints..path(
          style: PaintingStyle.fill,
          color: _ink,
          includes: <Offset>[
            const Offset(0.5, 20),
            const Offset(59.5, 20),
            const Offset(30, 0.5),
            const Offset(30, 39.5),
            const Offset(59.5, 0.5),
            _topLeftCut(0.5),
            _topLeftCut(0.9),
            _bottomRightCut(0.5),
            _bottomRightCut(0.9),
          ],
          excludes: <Offset>[
            const Offset(-0.5, 20),
            const Offset(1.5, 20),
            const Offset(30, 20),
            _topLeftCut(-0.2),
            _topLeftCut(1.2),
            _bottomRightCut(1.2),
          ],
        ),
      );
    });

    testWidgets('keeps the cut as wide as a wider side', (
      WidgetTester tester,
    ) async {
      await _pump(tester, const BorderSide(color: _ink, width: 2));

      expect(
        find.byKey(_box),
        paints..path(
          style: PaintingStyle.fill,
          includes: <Offset>[
            const Offset(1.5, 20),
            _topLeftCut(1.8),
            _bottomRightCut(1.8),
          ],
          excludes: <Offset>[
            const Offset(2.5, 20),
            _topLeftCut(2.2),
            _bottomRightCut(2.2),
          ],
        ),
      );
    });

    testWidgets('an outside side runs its band outside the box', (
      WidgetTester tester,
    ) async {
      await _pump(
        tester,
        const BorderSide(
          color: _ink,
          strokeAlign: BorderSide.strokeAlignOutside,
        ),
      );

      expect(
        find.byKey(_box),
        paints..path(
          style: PaintingStyle.fill,
          includes: <Offset>[
            const Offset(-0.5, 20),
            _topLeftCut(-0.5),
            _topLeftCut(-0.9),
          ],
          excludes: <Offset>[
            const Offset(0.5, 20),
            _topLeftCut(0.2),
            _topLeftCut(-1.2),
          ],
        ),
      );
    });

    testWidgets('a centred side straddles the edge', (
      WidgetTester tester,
    ) async {
      await _pump(
        tester,
        const BorderSide(
          color: _ink,
          strokeAlign: BorderSide.strokeAlignCenter,
        ),
      );

      expect(
        find.byKey(_box),
        paints..path(
          style: PaintingStyle.fill,
          includes: <Offset>[
            const Offset(-0.4, 20),
            const Offset(0.4, 20),
            _topLeftCut(-0.4),
            _topLeftCut(0.4),
          ],
          excludes: <Offset>[
            const Offset(-0.6, 20),
            const Offset(0.6, 20),
            _topLeftCut(-0.6),
            _topLeftCut(0.6),
          ],
        ),
      );
    });

    testWidgets('an infinite corner paints the clamped outline', (
      WidgetTester tester,
    ) async {
      await _pump(
        tester,
        const BorderSide(color: _ink),
        radius: const BorderRadius.all(Radius.circular(double.infinity)),
      );
      const Offset left = Offset(0, 20);
      const Offset top = Offset(30, 0);

      expect(
        find.byKey(_box),
        paints..path(
          style: PaintingStyle.fill,
          includes: <Offset>[_along(left, top, 0.5)],
          excludes: <Offset>[_along(left, top, 1.2), _along(left, top, -0.2)],
        ),
      );
    });

    testWidgets('a hairline side strokes the outline', (
      WidgetTester tester,
    ) async {
      await _pump(tester, const BorderSide(color: _ink, width: 0));

      expect(
        find.byKey(_box),
        paints..path(style: PaintingStyle.stroke, strokeWidth: 0, color: _ink),
      );
    });

    testWidgets('no side paints nothing', (WidgetTester tester) async {
      await _pump(tester, BorderSide.none);

      expect(find.byKey(_box), paintsNothing);
    });

    testWidgets('a one pixel side lights one pixel row inside the box', (
      WidgetTester tester,
    ) async {
      const Key frame = ValueKey<String>('frame');
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: RepaintBoundary(
              key: frame,
              child: ColoredBox(
                color: _ground,
                child: Padding(
                  padding: const EdgeInsets.all(10),
                  child: SizedBox(
                    width: _rect.width,
                    height: _rect.height,
                    child: DecoratedBox(
                      decoration: ShapeDecoration(
                        color: _ground,
                        shape: CutBorder(
                          side: const BorderSide(color: _ink),
                          borderRadius: _bevel(10),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );

      final List<Color> row =
          await pixelsAt(tester, find.byKey(frame), const <Offset>[
            Offset(9.5, 30),
            Offset(10.5, 30),
            Offset(11.5, 30),
            Offset(68.5, 30),
            Offset(69.5, 30),
            Offset(70.5, 30),
          ], ratio: 1);

      expect(row, <Matcher>[
        isPixel(_ground),
        isPixel(_ink),
        isPixel(_ground),
        isPixel(_ground),
        isPixel(_ink),
        isPixel(_ground),
      ]);
    });
  });

  test('the inner path runs a side\'s width inside every edge', () {
    final Path inner = CutBorder(
      side: const BorderSide(color: _ink),
      borderRadius: _bevel(10),
    ).getInnerPath(_rect);

    expect(inner.contains(const Offset(1.1, 20)), isTrue);
    expect(inner.contains(const Offset(0.9, 20)), isFalse);
    expect(inner.contains(const Offset(30, 38.9)), isTrue);
    expect(inner.contains(const Offset(30, 39.1)), isFalse);
    expect(inner.contains(_topLeftCut(1.1)), isTrue);
    expect(inner.contains(_topLeftCut(0.9)), isFalse);
    expect(inner.contains(_bottomRightCut(1.1)), isTrue);
    expect(inner.contains(_bottomRightCut(0.9)), isFalse);
  });

  test('an empty box has an empty inner path', () {
    final Path inner = CutBorder(
      side: const BorderSide(color: _ink),
      borderRadius: _bevel(10),
    ).getInnerPath(const Rect.fromLTWH(0, 0, 60, 0));

    expect(inner.getBounds(), Rect.zero);
  });

  test('elliptical corners keep the band a side\'s width', () {
    final Path inner = const CutBorder(
      side: BorderSide(color: _ink),
      borderRadius: BorderRadius.only(topLeft: Radius.elliptical(20, 10)),
    ).getInnerPath(_rect);
    const Offset left = Offset(0, 10);
    const Offset top = Offset(20, 0);

    expect(inner.contains(_along(left, top, 1.1)), isTrue);
    expect(inner.contains(_along(left, top, 0.9)), isFalse);
  });

  test('a corner clamped at half the box keeps the band a side\'s width', () {
    final Path inner = const CutBorder(
      side: BorderSide(color: _ink),
      borderRadius: BorderRadius.only(topLeft: Radius.circular(30)),
    ).getInnerPath(_rect);
    const Offset left = Offset(0, 20);
    const Offset top = Offset(30, 0);

    expect(inner.contains(_along(left, top, 1.1)), isTrue);
    expect(inner.contains(_along(left, top, 0.9)), isFalse);
  });

  group('changes keep the corrected border', () {
    final CutBorder border = CutBorder(
      side: const BorderSide(color: _ink),
      borderRadius: _bevel(10),
    );

    test('a copy is a cut border', () {
      const BorderSide side = BorderSide(color: _ground, width: 2);

      expect(
        border.copyWith(side: side),
        CutBorder(side: side, borderRadius: _bevel(10)),
      );
      expect(
        border.copyWith(borderRadius: _bevel(4)),
        CutBorder(
          side: const BorderSide(color: _ink),
          borderRadius: _bevel(4),
        ),
      );
      expect(border.copyWith(), border);
    });

    test('a scaled border is a cut border', () {
      expect(
        border.scale(2),
        CutBorder(
          side: const BorderSide(color: _ink, width: 2),
          borderRadius: _bevel(20),
        ),
      );
    });

    test('a border lerped into it is a cut border', () {
      final CutBorder wide = CutBorder(
        side: const BorderSide(color: _ink, width: 3),
        borderRadius: _bevel(20),
      );
      final CutBorder half = CutBorder(
        side: const BorderSide(color: _ink, width: 2),
        borderRadius: _bevel(15),
      );

      expect(ShapeBorder.lerp(border, wide, 0.5), half);
      expect(
        ShapeBorder.lerp(
          BeveledRectangleBorder(
            side: const BorderSide(color: _ink),
            borderRadius: _bevel(10),
          ),
          wide,
          0.5,
        ),
        half,
      );
      expect(ShapeBorder.lerp(null, border, 1), isA<CutBorder>());
    });

    test('a border lerped out of it is a cut border', () {
      final CutBorder half = CutBorder(
        side: const BorderSide(color: _ink, width: 2),
        borderRadius: _bevel(15),
      );

      expect(
        border.lerpTo(
          CutBorder(
            side: const BorderSide(color: _ink, width: 3),
            borderRadius: _bevel(20),
          ),
          0.5,
        ),
        half,
      );
      expect(
        border.lerpTo(
          BeveledRectangleBorder(
            side: const BorderSide(color: _ink, width: 3),
            borderRadius: _bevel(20),
          ),
          0.5,
        ),
        half,
      );
      expect(border.lerpTo(null, 1), isA<CutBorder>());
    });
  });
}
