import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/tokens.dart';

@immutable
class Corner {
  const Corner(this.radius, this.cut);

  final Radius radius;
  final double cut;
}

abstract final class Corners {
  static const Corner panel = Corner(Radii.md, 14);
  static const Corner shell = Corner(Radii.lg, 14);
  static const Corner hubCard = Corner(Radii.md, 12);
  static const Corner card = Corner(Radii.md, 10);
  static const Corner inset = Corner(Radii.sm, 10);
  static const Corner button = Corner(Radii.sm, 8);
  static const Corner tab = Corner(Radii.md, 8);
  static const Corner row = Corner(Radii.sm, 7);
  static const Corner chip = Corner(Radii.pill, 6);
  static const Corner tag = Corner(Radii.sm, 6);
  static const Corner badge = Corner(Radii.pill, 5);
  static const Corner label = Corner(Radii.sm, 5);
  static const Corner bar = Corner(Radii.pill, 0);
  static const Corner skeletonBar = Corner(Radius.circular(4), 0);
  static const Corner field = Corner(Radii.sm, 0);
  static const Corner check = Corner(Radius.circular(3), 0);
  static const Corner none = Corner(Radius.zero, 0);
}

abstract final class Marks {
  static const Size watched = Size(28, 24);
  static const Size playhead = Size(16, 14);
  static const Size playheadHalo = Size(24, 21);
  static const Size playButton = Size(48, 42);
  static const Size skipButton = Size(40, 35);
  static const Size node = Size(13, 11);
  static const Size readout = Size(44, 38);
  static const Size readoutDot = Size(6, 5);
}

enum CornerPart { all, leading, trailing, top }

class CutBorder extends BeveledRectangleBorder {
  const CutBorder({super.side, super.borderRadius});

  @override
  CutBorder copyWith({BorderSide? side, BorderRadiusGeometry? borderRadius}) =>
      CutBorder(
        side: side ?? this.side,
        borderRadius: borderRadius ?? this.borderRadius,
      );

  @override
  ShapeBorder scale(double t) =>
      CutBorder(side: side.scale(t), borderRadius: borderRadius * t);

  @override
  ShapeBorder? lerpFrom(ShapeBorder? a, double t) {
    if (a is BeveledRectangleBorder) {
      return CutBorder(
        side: BorderSide.lerp(a.side, side, t),
        borderRadius: BorderRadiusGeometry.lerp(
          a.borderRadius,
          borderRadius,
          t,
        )!,
      );
    }
    return super.lerpFrom(a, t);
  }

  @override
  ShapeBorder? lerpTo(ShapeBorder? b, double t) {
    if (b is BeveledRectangleBorder) {
      return CutBorder(
        side: BorderSide.lerp(side, b.side, t),
        borderRadius: BorderRadiusGeometry.lerp(
          borderRadius,
          b.borderRadius,
          t,
        )!,
      );
    }
    return super.lerpTo(b, t);
  }

  @override
  Path getInnerPath(Rect rect, {TextDirection? textDirection}) =>
      rect.isEmpty ? Path() : _edge(rect, textDirection, -side.strokeInset);

  @override
  void paint(Canvas canvas, Rect rect, {TextDirection? textDirection}) {
    if (rect.isEmpty || side.style == BorderStyle.none) {
      return;
    }
    if (side.width == 0) {
      canvas.drawPath(
        getOuterPath(rect, textDirection: textDirection),
        side.toPaint(),
      );
      return;
    }
    canvas.drawPath(
      _edge(rect, textDirection, side.strokeOutset)
        ..fillType = PathFillType.evenOdd
        ..addPath(
          getInnerPath(rect, textDirection: textDirection),
          Offset.zero,
        ),
      Paint()..color = side.color,
    );
  }

  Path _edge(Rect rect, TextDirection? textDirection, double offset) {
    final List<Offset> corners = _corners(
      borderRadius.resolve(textDirection).toRRect(rect),
    );
    final int n = corners.length;
    return Path()..addPolygon(<Offset>[
      for (int i = 0; i < n; i++)
        corners[i] +
            _miter(corners[(i + n - 1) % n], corners[i], corners[(i + 1) % n]) *
                offset,
    ], true);
  }

  static Offset _outward(Offset from, Offset to) {
    final Offset run = to - from;
    return Offset(run.dy, -run.dx) / run.distance;
  }

  static Offset _miter(Offset before, Offset at, Offset after) {
    final Offset a = _outward(before, at);
    final Offset b = _outward(at, after);
    return (a + b) / (1 + a.dx * b.dx + a.dy * b.dy);
  }

  static List<Offset> _corners(RRect r) {
    final double midX = r.center.dx;
    final double midY = r.center.dy;
    final List<Offset> all = <Offset>[
      Offset(r.left, math.min(midY, r.top + r.tlRadiusY)),
      Offset(math.min(midX, r.left + r.tlRadiusX), r.top),
      Offset(math.max(midX, r.right - r.trRadiusX), r.top),
      Offset(r.right, math.min(midY, r.top + r.trRadiusY)),
      Offset(r.right, math.max(midY, r.bottom - r.brRadiusY)),
      Offset(math.max(midX, r.right - r.brRadiusX), r.bottom),
      Offset(math.min(midX, r.left + r.blRadiusX), r.bottom),
      Offset(r.left, math.max(midY, r.bottom - r.blRadiusY)),
    ];
    return <Offset>[
      for (int i = 0; i < all.length; i++)
        if (all[i] != all[(i + 1) % all.length]) all[i],
    ];
  }
}

const String kMonoFamily = 'monospace';
const List<String> kMonoFallback = <String>[
  'SF Mono',
  'Menlo',
  'Consolas',
  'DejaVu Sans Mono',
  'Liberation Mono',
  'Courier New',
];

@immutable
class Shapes {
  const Shapes.rounded({this.highContrast = false})
    : cut = false,
      _line = null,
      _lineSoft = null,
      _wash = null,
      _panel = null,
      _track = null;

  Shapes.cut(Tokens t, {required this.highContrast})
    : cut = true,
      _line = highContrast ? t.accent : t.accent.withValues(alpha: 0.62),
      _lineSoft = highContrast ? t.accent : t.accent.withValues(alpha: 0.26),
      _wash = Color.alphaBlend(t.accent.withValues(alpha: 0.16), t.surface),
      _panel = t.surface.withValues(alpha: 0.9),
      _track = t.accent.withValues(alpha: 0.26);

  static Shapes of(
    AppThemeVariant variant,
    Tokens t, {
    bool highContrast = false,
  }) => variant == AppThemeVariant.hologram
      ? Shapes.cut(t, highContrast: highContrast)
      : Shapes.rounded(highContrast: highContrast);

  final bool cut;
  final bool highContrast;
  final Color? _line;
  final Color? _lineSoft;
  final Color? _wash;
  final Color? _panel;
  final Color? _track;

  Color line(Color today) => _line ?? today;
  Color lineSoft(Color today) => _lineSoft ?? today;
  Color wash(Color today) => _wash ?? today;
  Color panel(Color today) => _panel ?? today;
  Color track(Color today) => _track ?? today;

  TextStyle mono(TextStyle style) => cut
      ? style.copyWith(
          fontFamily: kMonoFamily,
          fontFamilyFallback: kMonoFallback,
        )
      : style;
  String caps(String text) => cut ? text.toUpperCase() : text;
  String? capsLabel(String text) => cut ? text : null;

  BorderRadius radii(Corner corner, {CornerPart part = CornerPart.all}) {
    if (!cut) {
      final Radius r = corner.radius;
      return switch (part) {
        CornerPart.all => BorderRadius.all(r),
        CornerPart.leading => BorderRadius.horizontal(left: r),
        CornerPart.trailing => BorderRadius.horizontal(right: r),
        CornerPart.top => BorderRadius.vertical(top: r),
      };
    }
    final Radius r = Radius.circular(corner.cut);
    return switch (part) {
      CornerPart.all => BorderRadius.only(topLeft: r, bottomRight: r),
      CornerPart.leading || CornerPart.top => BorderRadius.only(topLeft: r),
      CornerPart.trailing => BorderRadius.only(bottomRight: r),
    };
  }

  OutlinedBorder border(
    Corner corner, {
    BorderSide side = BorderSide.none,
    CornerPart part = CornerPart.all,
  }) => cut
      ? CutBorder(
          borderRadius: radii(corner, part: part),
          side: side,
        )
      : RoundedRectangleBorder(
          borderRadius: radii(corner, part: part),
          side: side,
        );

  Decoration box(
    Corner corner, {
    Color? color,
    BorderSide? side,
    List<BoxShadow>? shadows,
    Gradient? gradient,
    CornerPart part = CornerPart.all,
  }) => cut
      ? ShapeDecoration(
          color: color,
          gradient: gradient,
          shadows: shadows,
          shape: border(corner, side: side ?? BorderSide.none, part: part),
        )
      : BoxDecoration(
          color: color,
          gradient: gradient,
          borderRadius: radii(corner, part: part),
          border: side == null ? null : Border.fromBorderSide(side),
          boxShadow: shadows,
        );

  Widget clip(Corner corner, {required Widget child}) {
    if (!cut) {
      return ClipRRect(borderRadius: radii(corner), child: child);
    }
    if (corner.cut == 0) {
      return ClipRect(child: child);
    }
    return ClipPath(
      clipper: ShapeBorderClipper(shape: border(corner)),
      child: child,
    );
  }

  static OutlinedBorder hexagon(Size size) => BeveledRectangleBorder(
    borderRadius: BorderRadius.all(
      Radius.elliptical(size.width / 4, size.height / 2),
    ),
  );

  OutlinedBorder round({BorderSide side = BorderSide.none}) => cut
      ? StarBorder.polygon(sides: 6, rotation: 30, side: side)
      : CircleBorder(side: side);

  Decoration roundBox({Color? color, BorderSide? side}) => cut
      ? ShapeDecoration(
          color: color,
          shape: round(side: side ?? BorderSide.none),
        )
      : BoxDecoration(
          color: color,
          shape: BoxShape.circle,
          border: side == null ? null : Border.fromBorderSide(side),
        );
}
