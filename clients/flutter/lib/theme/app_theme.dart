import 'package:flutter/foundation.dart';
import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/theme/app_button.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_ext.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_ext.dart';

const double _kFieldTextHeight = 24;
const double _kFieldPadding = (kControlHeight - _kFieldTextHeight) / 2;

const String? _familySans = kIsWeb ? 'system-ui' : null;
const List<String> _sansFallback = kIsWeb
    ? <String>[
        '-apple-system',
        'Segoe UI',
        'Roboto',
        'Helvetica',
        'Arial',
        'sans-serif',
      ]
    : <String>[];

TextTheme _textTheme(Tokens t) {
  TextStyle style(double size, double height, FontWeight weight) => TextStyle(
    fontSize: size,
    height: height,
    fontWeight: weight,
    color: t.text,
    fontFamily: _familySans,
    fontFamilyFallback: _sansFallback,
  );

  return TextTheme(
    headlineLarge: style(25.6, 1.2, FontWeight.w700),
    headlineMedium: style(20, 1.25, FontWeight.w600),
    titleMedium: style(16.8, 1.35, FontWeight.w600),
    bodyLarge: style(16, 1.5, FontWeight.w400),
    bodyMedium: style(14, 1.45, FontWeight.w400),
    bodySmall: style(12, 1.4, FontWeight.w400),
    labelLarge: style(14, 1.2, FontWeight.w600),
  );
}

const TextStyle monoStyle = TextStyle(
  fontFeatures: <FontFeature>[FontFeature.tabularFigures()],
);

ThemeData buildTheme(AppThemeVariant variant, {bool highContrast = false}) {
  final Tokens t = Tokens.of(variant, highContrast: highContrast);
  final ColorScheme scheme = ColorScheme(
    brightness: t.isDark ? Brightness.dark : Brightness.light,
    primary: t.accent,
    onPrimary: t.accentContrast,
    secondary: t.accent,
    onSecondary: t.accentContrast,
    surface: t.surface,
    onSurface: t.text,
    error: t.danger,
    onError: t.accentContrast,
    outline: t.border,
    surfaceTint: Colors.transparent,
  );

  final Shapes shapes = Shapes.of(variant, t, highContrast: highContrast);
  final OutlinedBorder buttonShape = shapes.border(Corners.button);
  final BorderRadius fieldRadius = shapes.radii(Corners.field);
  const EdgeInsets buttonPadding = EdgeInsets.symmetric(
    horizontal: kButtonPaddingX,
    vertical: kButtonPaddingY,
  );
  const TextStyle buttonText = TextStyle(
    fontSize: 14,
    fontWeight: FontWeight.w600,
  );

  return ThemeData(
    useMaterial3: true,
    visualDensity: VisualDensity.standard,
    colorScheme: scheme,
    scaffoldBackgroundColor: t.bg,
    canvasColor: t.bg,
    dividerColor: t.border,
    textTheme: _textTheme(t),
    fontFamily: _familySans,
    fontFamilyFallback: _sansFallback,
    extensions: <ThemeExtension<dynamic>>[TokensExt(t), ShapesExt(shapes)],
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: t.surface,
      hintStyle: TextStyle(color: t.muted),
      contentPadding: const EdgeInsets.symmetric(
        horizontal: Space.s2,
        vertical: _kFieldPadding,
      ),
      border: OutlineInputBorder(borderRadius: fieldRadius),
      enabledBorder: OutlineInputBorder(
        borderRadius: fieldRadius,
        borderSide: BorderSide(color: t.fieldBorder),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: fieldRadius,
        borderSide: BorderSide(color: t.accent, width: 2),
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        shape: buttonShape,
        padding: buttonPadding,
        textStyle: buttonText,
        elevation: 0,
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: t.accent,
        shape: buttonShape,
        padding: buttonPadding,
        textStyle: buttonText,
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: t.text,
        backgroundColor: t.surface,
        side: BorderSide(color: shapes.line(t.border)),
        shape: buttonShape,
        padding: buttonPadding,
        textStyle: buttonText,
      ),
    ),
    iconButtonTheme: shapes.cut
        ? IconButtonThemeData(
            style: ButtonStyle(
              shape: WidgetStatePropertyAll<OutlinedBorder>(shapes.round()),
            ),
          )
        : null,
    chipTheme: shapes.cut
        ? ChipThemeData(shape: shapes.border(Corners.chip))
        : null,
    bottomSheetTheme: shapes.cut
        ? BottomSheetThemeData(
            shape: shapes.border(Corners.panel, part: CornerPart.top),
          )
        : null,
    dialogTheme: DialogThemeData(
      backgroundColor: t.surface,
      surfaceTintColor: Colors.transparent,
      elevation: 8,
      shape: shapes.border(Corners.panel),
    ),
    popupMenuTheme: PopupMenuThemeData(
      color: t.surface,
      surfaceTintColor: Colors.transparent,
      elevation: 8,
      shape: shapes.border(
        Corners.panel,
        side: BorderSide(color: shapes.line(t.border)),
      ),
    ),
    tooltipTheme: TooltipThemeData(
      waitDuration: const Duration(milliseconds: 400),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      margin: const EdgeInsets.all(4),
      decoration: shapes.box(
        Corners.tag,
        color: t.surfaceAlt,
        side: BorderSide(color: shapes.line(t.border)),
        shadows: <BoxShadow>[
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.35),
            blurRadius: 12,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      textStyle: TextStyle(
        color: t.text,
        fontSize: 12,
        height: 1.3,
        fontWeight: FontWeight.w500,
        fontFamily: _familySans,
        fontFamilyFallback: _sansFallback,
      ),
    ),
    checkboxTheme: CheckboxThemeData(
      fillColor: WidgetStateProperty.resolveWith<Color>(
        (Set<WidgetState> states) =>
            states.contains(WidgetState.selected) ? t.accent : t.surface,
      ),
      checkColor: WidgetStateProperty.all<Color>(t.accentContrast),
      side: BorderSide(color: t.border, width: 1.5),
      shape: shapes.border(Corners.check),
    ),
  );
}
