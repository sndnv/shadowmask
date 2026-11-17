import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/resume_ring.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

enum _Look { rounded, cut, plain }

bool _roundedRadius(BorderRadiusGeometry? radius) =>
    radius != null &&
    radius != BorderRadius.zero &&
    radius != BorderRadiusDirectional.zero;

_Look _radiusLook(BorderRadiusGeometry? radius) =>
    _roundedRadius(radius) ? _Look.rounded : _Look.plain;

_Look _borderLook(ShapeBorder? shape) => switch (shape) {
  BeveledRectangleBorder() || StarBorder() => _Look.cut,
  RoundedRectangleBorder(:final BorderRadiusGeometry borderRadius) =>
    _radiusLook(borderRadius),
  ContinuousRectangleBorder(:final BorderRadiusGeometry borderRadius) =>
    _radiusLook(borderRadius),
  RoundedSuperellipseBorder(:final BorderRadiusGeometry borderRadius) =>
    _radiusLook(borderRadius),
  CircleBorder() || StadiumBorder() || OvalBorder() => _Look.rounded,
  _ => _Look.plain,
};

_Look _decorationLook(Decoration? decoration) => switch (decoration) {
  BoxDecoration(
    :final BoxShape shape,
    :final BorderRadiusGeometry? borderRadius,
  ) =>
    shape == BoxShape.circle || _roundedRadius(borderRadius)
        ? _Look.rounded
        : _Look.plain,
  ShapeDecoration(:final ShapeBorder shape) => _borderLook(shape),
  _ => _Look.plain,
};

_Look _clipperLook(CustomClipper<Path>? clipper) =>
    clipper is ShapeBorderClipper ? _borderLook(clipper.shape) : _Look.plain;

_Look _materialLook(Material material) {
  if (material.shape != null) {
    return _borderLook(material.shape);
  }
  if (material.borderRadius != null) {
    return _radiusLook(material.borderRadius);
  }
  return switch (material.type) {
    MaterialType.circle ||
    MaterialType.card ||
    MaterialType.button => _Look.rounded,
    _ => _Look.plain,
  };
}

_Look _inkLook(InkResponse ink) {
  if (ink.customBorder != null) {
    return _borderLook(ink.customBorder);
  }
  if (_roundedRadius(ink.borderRadius)) {
    return _Look.rounded;
  }
  return ink.highlightShape == BoxShape.circle ? _Look.rounded : _Look.plain;
}

_Look _lookOf(Widget widget) => switch (widget) {
  DecoratedBox(:final Decoration decoration) => _decorationLook(decoration),
  DecoratedSliver(:final Decoration decoration) => _decorationLook(decoration),
  Ink(:final Decoration? decoration) => _decorationLook(decoration),
  ClipRRect(:final BorderRadiusGeometry? borderRadius) => _radiusLook(
    borderRadius,
  ),
  ClipRSuperellipse(:final BorderRadiusGeometry? borderRadius) => _radiusLook(
    borderRadius,
  ),
  ClipOval() => _Look.rounded,
  ClipPath(:final CustomClipper<Path>? clipper) => _clipperLook(clipper),
  PhysicalShape(:final CustomClipper<Path> clipper) => _clipperLook(clipper),
  PhysicalModel(:final BoxShape shape, :final BorderRadius? borderRadius) =>
    shape == BoxShape.circle || _roundedRadius(borderRadius)
        ? _Look.rounded
        : _Look.plain,
  Material() => _materialLook(widget),
  InkResponse() => _inkLook(widget),
  _ => _Look.plain,
};

bool _chipDeleteButton(Element element) =>
    element.widget is InkWell &&
    (element.widget as InkWell).customBorder == const CircleBorder() &&
    element.findAncestorWidgetOfExactType<RawChip>() != null;

bool _resumeRing(Element element) =>
    element.findAncestorWidgetOfExactType<ResumeRing>() != null;

void expectShapesOf(WidgetTester tester, AppThemeVariant variant) {
  final _Look foreign = variant == AppThemeVariant.hologram
      ? _Look.rounded
      : _Look.cut;
  final List<String> found = <String>[
    for (final Element element in tester.allElements)
      if (_lookOf(element.widget) == foreign &&
          !_chipDeleteButton(element) &&
          !_resumeRing(element))
        element.debugGetCreatorChain(8),
  ];
  expect(
    found,
    isEmpty,
    reason: variant == AppThemeVariant.hologram
        ? 'hologram still draws these rounded'
        : '${variant.name} draws these cut',
  );
}
