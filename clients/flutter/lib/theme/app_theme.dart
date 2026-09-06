import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:shadowmask/theme/app_button.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/radii.dart';
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
const String? _familyMono = kIsWeb ? 'ui-monospace' : null;
const List<String> _monoFallback = <String>[
  'SF Mono',
  'SFMono-Regular',
  'Menlo',
  'Consolas',
  'monospace',
];

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
  fontFamily: _familyMono,
  fontFamilyFallback: _monoFallback,
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

  const RoundedRectangleBorder buttonShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.all(Radii.sm),
  );
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
    extensions: <ThemeExtension<dynamic>>[TokensExt(t)],
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: t.surface,
      hintStyle: TextStyle(color: t.muted),
      contentPadding: const EdgeInsets.symmetric(
        horizontal: Space.s2,
        vertical: _kFieldPadding,
      ),
      border: const OutlineInputBorder(
        borderRadius: BorderRadius.all(Radii.sm),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: const BorderRadius.all(Radii.sm),
        borderSide: BorderSide(color: t.fieldBorder),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: const BorderRadius.all(Radii.sm),
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
        side: BorderSide(color: t.border),
        shape: buttonShape,
        padding: buttonPadding,
        textStyle: buttonText,
      ),
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: t.surface,
      surfaceTintColor: Colors.transparent,
      elevation: 8,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.all(Radii.md),
      ),
    ),
    popupMenuTheme: PopupMenuThemeData(
      color: t.surface,
      surfaceTintColor: Colors.transparent,
      elevation: 8,
      shape: RoundedRectangleBorder(
        borderRadius: const BorderRadius.all(Radii.md),
        side: BorderSide(color: t.border),
      ),
    ),
    tooltipTheme: TooltipThemeData(
      waitDuration: const Duration(milliseconds: 400),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      margin: const EdgeInsets.all(4),
      decoration: BoxDecoration(
        color: t.surfaceAlt,
        borderRadius: const BorderRadius.all(Radii.sm),
        border: Border.all(color: t.border),
        boxShadow: <BoxShadow>[
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
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.all(Radius.circular(3)),
      ),
    ),
  );
}
