import 'package:flutter/material.dart';

import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';

const Offset kMenuOffset = Offset(0, Space.s1);

MenuStyle appMenuStyle(Tokens t, {double? minWidth}) => MenuStyle(
  backgroundColor: WidgetStatePropertyAll<Color>(t.surface),
  surfaceTintColor: const WidgetStatePropertyAll<Color>(Colors.transparent),
  minimumSize: minWidth == null
      ? null
      : WidgetStatePropertyAll<Size>(Size(minWidth, 0)),
  shape: WidgetStatePropertyAll<OutlinedBorder>(
    RoundedRectangleBorder(
      borderRadius: const BorderRadius.all(Radii.md),
      side: BorderSide(color: t.border),
    ),
  ),
  padding: const WidgetStatePropertyAll<EdgeInsetsGeometry>(
    EdgeInsets.symmetric(vertical: Space.s2),
  ),
);
