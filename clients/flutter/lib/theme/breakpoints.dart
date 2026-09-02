import 'package:flutter/widgets.dart';

abstract final class Breakpoints {
  static const double sm = 560;
  static const double md = 900;
  static const double lg = 1072;
}

bool compactViewport(BuildContext context) {
  final Size size = MediaQuery.sizeOf(context);
  return size.width < Breakpoints.sm || size.height < Breakpoints.sm;
}
