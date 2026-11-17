import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/theme/shapes.dart';

class ShapesExt extends ThemeExtension<ShapesExt> {
  const ShapesExt(this.shapes);

  final Shapes shapes;

  @override
  ShapesExt copyWith({Shapes? shapes}) => ShapesExt(shapes ?? this.shapes);

  @override
  ShapesExt lerp(ThemeExtension<ShapesExt>? other, double v) => this;
}
