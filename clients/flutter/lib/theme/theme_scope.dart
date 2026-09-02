import 'package:flutter/widgets.dart';

import 'package:shadowmask/theme/app_theme_variant.dart';

class ThemeScope extends InheritedWidget {
  const ThemeScope({
    super.key,
    required this.variant,
    required this.setVariant,
    required super.child,
    this.highContrast = false,
    this.setHighContrast = _ignore,
  });

  final AppThemeVariant variant;
  final ValueChanged<AppThemeVariant> setVariant;
  final bool highContrast;
  final ValueChanged<bool> setHighContrast;

  static void _ignore(bool _) {}

  static ThemeScope of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<ThemeScope>()!;

  @override
  bool updateShouldNotify(ThemeScope oldWidget) =>
      variant != oldWidget.variant || highContrast != oldWidget.highContrast;
}
