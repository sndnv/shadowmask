import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_ext.dart';

extension ShapesContext on BuildContext {
  Shapes get shapes => Theme.of(this).extension<ShapesExt>()!.shapes;
}
