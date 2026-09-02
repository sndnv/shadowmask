import 'package:flutter/material.dart';

import 'package:shadowmask/theme/space.dart';

const double kButtonPaddingX = 14;
const double kButtonPaddingY = 10;

const ButtonStyle kFieldButtonStyle = ButtonStyle(
  padding: WidgetStatePropertyAll<EdgeInsetsGeometry>(
    EdgeInsets.symmetric(horizontal: kButtonPaddingX),
  ),
  minimumSize: WidgetStatePropertyAll<Size>(Size(0, kControlHeight)),
  fixedSize: WidgetStatePropertyAll<Size>(Size.fromHeight(kControlHeight)),
  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
);
