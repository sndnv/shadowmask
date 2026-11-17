import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/tokens.dart';

const Shapes _rounded = Shapes.rounded();
final Shapes _cut = Shapes.cut(Tokens.hologram, highContrast: false);

BorderRadius _bevel(double cut) => BorderRadius.only(
  topLeft: Radius.circular(cut),
  bottomRight: Radius.circular(cut),
);

void main() {
  group('the rounded set hands back what the other themes draw', () {
    test('a border is a rounded rectangle on the corner\'s radius', () {
      expect(
        _rounded.border(Corners.button),
        const RoundedRectangleBorder(borderRadius: BorderRadius.all(Radii.sm)),
      );
      expect(
        _rounded.border(Corners.panel, side: const BorderSide(width: 2)),
        const RoundedRectangleBorder(
          borderRadius: BorderRadius.all(Radii.md),
          side: BorderSide(width: 2),
        ),
      );
    });

    test('the parts round the sides they rounded before', () {
      expect(
        _rounded.radii(Corners.button, part: CornerPart.leading),
        const BorderRadius.horizontal(left: Radii.sm),
      );
      expect(
        _rounded.radii(Corners.button, part: CornerPart.trailing),
        const BorderRadius.horizontal(right: Radii.sm),
      );
      expect(
        _rounded.radii(Corners.tab, part: CornerPart.top),
        const BorderRadius.only(topLeft: Radii.md, topRight: Radii.md),
      );
    });

    test('a box is the box decoration it was', () {
      const Color fill = Color(0xFF102030);
      const Color edge = Color(0xFF405060);
      expect(
        _rounded.box(
          Corners.panel,
          color: fill,
          side: const BorderSide(color: edge),
        ),
        BoxDecoration(
          color: fill,
          borderRadius: const BorderRadius.all(Radii.md),
          border: Border.all(color: edge),
        ),
      );
      expect(
        _rounded.box(Corners.inset, color: fill),
        const BoxDecoration(
          color: fill,
          borderRadius: BorderRadius.all(Radii.sm),
        ),
      );
    });

    test('a box keeps its shadows and rounds only its part', () {
      const List<BoxShadow> shadows = <BoxShadow>[
        BoxShadow(color: Color(0x80000000), blurRadius: 28),
      ];

      expect(
        _rounded.box(Corners.card, color: Colors.black, shadows: shadows),
        const BoxDecoration(
          color: Colors.black,
          borderRadius: BorderRadius.all(Radii.md),
          boxShadow: shadows,
        ),
      );
      expect(
        _rounded.box(Corners.tab, part: CornerPart.top),
        const BoxDecoration(borderRadius: BorderRadius.vertical(top: Radii.md)),
      );
    });

    test('a clip is a rounded clip', () {
      final Widget clip = _rounded.clip(Corners.card, child: const SizedBox());

      expect(clip, isA<ClipRRect>());
      expect(
        (clip as ClipRRect).borderRadius,
        const BorderRadius.all(Radii.md),
      );
    });

    test('a round shape is a circle', () {
      expect(_rounded.round(), const CircleBorder());
      expect(
        _rounded.roundBox(color: const Color(0xFF00FF00)),
        const BoxDecoration(color: Color(0xFF00FF00), shape: BoxShape.circle),
      );
    });

    test('fields and checkboxes keep their small radius', () {
      expect(_rounded.radii(Corners.field), const BorderRadius.all(Radii.sm));
      expect(
        _rounded.border(Corners.check),
        const RoundedRectangleBorder(
          borderRadius: BorderRadius.all(Radius.circular(3)),
        ),
      );
    });

    test('the line colours are the colours passed in', () {
      const Color today = Color(0xFF263338);

      expect(_rounded.line(today), today);
      expect(_rounded.lineSoft(today), today);
      expect(_rounded.wash(today), today);
      expect(_rounded.panel(today), today);
      expect(_rounded.track(today), today);
    });

    test('the rounded set carries the contrast it was made for', () {
      expect(_rounded.highContrast, isFalse);
      expect(
        Shapes.of(
          AppThemeVariant.dark,
          Tokens.dark,
          highContrast: true,
        ).highContrast,
        isTrue,
      );
    });
  });

  group('tracks', () {
    test('a hologram track is the accent at 26% whatever the contrast', () {
      const Color today = Color(0xFF263338);
      final Color soft = Tokens.hologram.accent.withValues(alpha: 0.26);
      final Shapes hard = Shapes.cut(Tokens.hologram, highContrast: true);

      expect(_cut.track(today), soft);
      expect(hard.track(today), soft);
      expect(_cut.highContrast, isFalse);
      expect(hard.highContrast, isTrue);
    });
  });

  group('the cut set', () {
    test('cuts the top-left and bottom-right corners by the role\'s size', () {
      expect(_cut.border(Corners.panel), CutBorder(borderRadius: _bevel(14)));
      expect(_cut.border(Corners.hubCard), CutBorder(borderRadius: _bevel(12)));
      expect(_cut.border(Corners.card), CutBorder(borderRadius: _bevel(10)));
      expect(_cut.border(Corners.inset), CutBorder(borderRadius: _bevel(10)));
      expect(_cut.border(Corners.button), CutBorder(borderRadius: _bevel(8)));
      expect(_cut.border(Corners.row), CutBorder(borderRadius: _bevel(7)));
      expect(_cut.border(Corners.chip), CutBorder(borderRadius: _bevel(6)));
      expect(_cut.border(Corners.badge), CutBorder(borderRadius: _bevel(5)));
      expect(_cut.border(Corners.shell), CutBorder(borderRadius: _bevel(14)));
      expect(_cut.border(Corners.tab), CutBorder(borderRadius: _bevel(8)));
      expect(_cut.border(Corners.tag), CutBorder(borderRadius: _bevel(6)));
      expect(_cut.border(Corners.label), CutBorder(borderRadius: _bevel(5)));
    });

    test('a half keeps only its own cut', () {
      expect(
        _cut.radii(Corners.button, part: CornerPart.leading),
        const BorderRadius.only(topLeft: Radius.circular(8)),
      );
      expect(
        _cut.radii(Corners.button, part: CornerPart.trailing),
        const BorderRadius.only(bottomRight: Radius.circular(8)),
      );
      expect(
        _cut.radii(Corners.tab, part: CornerPart.top),
        const BorderRadius.only(topLeft: Radius.circular(8)),
      );
    });

    test('bars, fields and checkboxes are square', () {
      expect(_cut.radii(Corners.bar), BorderRadius.zero);
      expect(_cut.radii(Corners.field), BorderRadius.zero);
      expect(_cut.radii(Corners.check), BorderRadius.zero);
    });

    test('a box is a shape decoration with the cut border', () {
      const Color fill = Color(0xFF102030);
      const BorderSide edge = BorderSide(color: Color(0xFF405060));
      final BoxShadow shadow = BoxShadow(
        color: Colors.black.withValues(alpha: 0.3),
        blurRadius: 4,
      );

      expect(
        _cut.box(
          Corners.panel,
          color: fill,
          side: edge,
          shadows: <BoxShadow>[shadow],
        ),
        ShapeDecoration(
          color: fill,
          shadows: <BoxShadow>[shadow],
          shape: CutBorder(borderRadius: _bevel(14), side: edge),
        ),
      );
    });

    test('a clip follows the cut border', () {
      final Widget clip = _cut.clip(Corners.card, child: const SizedBox());

      expect(clip, isA<ClipPath>());
      final CustomClipper<Path>? clipper = (clip as ClipPath).clipper;
      expect(clipper, isA<ShapeBorderClipper>());
      expect(
        (clipper! as ShapeBorderClipper).shape,
        CutBorder(borderRadius: _bevel(10)),
      );
    });

    test('a round shape is a regular flat-topped hexagon across its box', () {
      const Rect box = Rect.fromLTWH(0, 0, 40, 40);
      final Path path = _cut.round().getOuterPath(box);
      final double rise = 20 * math.sqrt(3) / 2;

      for (final Offset vertex in <Offset>[
        Offset(10, 20 - rise),
        Offset(30, 20 - rise),
        const Offset(40, 20),
        Offset(30, 20 + rise),
        Offset(10, 20 + rise),
        const Offset(0, 20),
      ]) {
        expect(path.contains(_inward(vertex, box.center)), isTrue);
        expect(path.contains(_outward(vertex, box.center)), isFalse);
      }
      expect(path.getBounds().width, closeTo(40, 0.05));
      expect(path.getBounds().height, closeTo(rise * 2, 0.05));
    });

    test('a round box is a hexagon shape decoration', () {
      final Decoration box = _cut.roundBox(color: const Color(0xFF00FF00));

      expect(box, isA<ShapeDecoration>());
      expect((box as ShapeDecoration).shape, _cut.round());
      expect(box.color, const Color(0xFF00FF00));
    });

    test('draws lines from the accent and fills from the surface', () {
      const Tokens t = Tokens.hologram;
      const Color today = Color(0xFF263338);

      expect(_cut.line(today), t.accent.withValues(alpha: 0.62));
      expect(_cut.lineSoft(today), t.accent.withValues(alpha: 0.26));
      expect(
        _cut.wash(today),
        Color.alphaBlend(t.accent.withValues(alpha: 0.16), t.surface),
      );
      expect(_cut.panel(today), t.surface.withValues(alpha: 0.9));
    });

    test('high contrast draws both lines in the full accent', () {
      final Shapes hard = Shapes.cut(
        Tokens.hologram.hardened(),
        highContrast: true,
      );

      expect(hard.line(Colors.black), Tokens.hologram.accent);
      expect(hard.lineSoft(Colors.black), Tokens.hologram.accent);
    });
  });

  group('labels', () {
    const TextStyle base = TextStyle(fontSize: 12, color: Color(0xFF102030));

    test('the rounded set leaves the face and the case alone', () {
      expect(_rounded.mono(base), base);
      expect(_rounded.caps('Version 1'), 'Version 1');
      expect(_rounded.capsLabel('Version 1'), isNull);
    });

    test('hologram sets labels in the system monospace face', () {
      final TextStyle mono = _cut.mono(base);

      expect(mono.fontFamily, kMonoFamily);
      expect(mono.fontFamilyFallback, kMonoFallback);
      expect(mono.fontSize, 12);
      expect(mono.color, const Color(0xFF102030));
    });

    test('hologram shows labels in capitals but reads them as written', () {
      expect(_cut.caps('Version 1'), 'VERSION 1');
      expect(_cut.capsLabel('Version 1'), 'Version 1');
    });
  });

  group('marks', () {
    for (final Size size in const <Size>[Marks.watched, Marks.playButton]) {
      test('a ${size.width.round()} by ${size.height.round()} hexagon fills '
          'its box', () {
        final Rect box = Offset.zero & size;
        final Path path = Shapes.hexagon(size).getOuterPath(box);
        final double w = size.width;
        final double h = size.height;

        for (final Offset vertex in <Offset>[
          Offset(w / 4, 0),
          Offset(w * 3 / 4, 0),
          Offset(w, h / 2),
          Offset(w * 3 / 4, h),
          Offset(w / 4, h),
          Offset(0, h / 2),
        ]) {
          expect(path.contains(_inward(vertex, box.center)), isTrue);
        }
        for (final (double x, double y) in <(double, double)>[
          (w / 8, h / 4),
          (w * 7 / 8, h / 4),
          (w * 7 / 8, h * 3 / 4),
          (w / 8, h * 3 / 4),
        ]) {
          final double inward = x < w / 2 ? 0.5 : -0.5;
          expect(path.contains(Offset(x + inward, y)), isTrue);
          expect(path.contains(Offset(x - inward, y)), isFalse);
        }
        expect(path.getBounds(), box);
      });
    }

    test('the marks take the mock\'s sizes', () {
      expect(Marks.watched, const Size(28, 24));
      expect(Marks.playhead, const Size(16, 14));
      expect(Marks.playheadHalo, const Size(24, 21));
      expect(Marks.playButton, const Size(48, 42));
      expect(Marks.node, const Size(13, 11));
    });
  });

  test('only hologram cuts', () {
    for (final AppThemeVariant variant in AppThemeVariant.values) {
      final Shapes shapes = Shapes.of(variant, Tokens.of(variant));
      expect(
        shapes.cut,
        variant == AppThemeVariant.hologram,
        reason: variant.name,
      );
    }
  });
}

Offset _inward(Offset vertex, Offset center) =>
    Offset.lerp(vertex, center, 0.05)!;

Offset _outward(Offset vertex, Offset center) =>
    Offset.lerp(vertex, center, -0.05)!;
