import 'package:flutter/widgets.dart';

import 'package:shadowmask/theme/app_theme_variant.dart';

@immutable
class Tokens {
  const Tokens({
    required this.bg,
    required this.surface,
    required this.surfaceAlt,
    required this.text,
    required this.muted,
    required this.border,
    required this.accent,
    required this.accentContrast,
    required this.ok,
    required this.warn,
    required this.danger,
    required this.okBg,
    required this.warnBg,
    required this.dangerBg,
    required this.artBg,
    required this.isDark,
    this.rowTintAlpha = 0.12,
    this.rowHoverAlpha = 0.06,
  });

  final Color bg;
  final Color surface;
  final Color surfaceAlt;
  final Color text;
  final Color muted;
  final Color border;
  final Color accent;
  final Color accentContrast;
  final Color ok;
  final Color warn;
  final Color danger;
  final Color okBg;
  final Color warnBg;
  final Color dangerBg;
  final Color artBg;
  final bool isDark;
  final double rowTintAlpha;
  final double rowHoverAlpha;

  static const Tokens dark = Tokens(
    bg: Color(0xFF0B1113),
    surface: Color(0xFF121A1D),
    surfaceAlt: Color(0xFF1A2529),
    text: Color(0xFFE6EEF0),
    muted: Color(0xFF90A3A8),
    border: Color(0xFF263338),
    accent: Color(0xFF35C9D6),
    accentContrast: Color(0xFF06171A),
    ok: Color(0xFF3FAE6F),
    warn: Color(0xFFD69A1E),
    danger: Color(0xFFE05A3C),
    okBg: Color(0xFF13271F),
    warnBg: Color(0xFF2A2410),
    dangerBg: Color(0xFF241310),
    artBg: Color(0xFF06090B),
    isDark: true,
  );

  static const Tokens light = Tokens(
    bg: Color(0xFFF5F7F8),
    surface: Color(0xFFFFFFFF),
    surfaceAlt: Color(0xFFEAF0F1),
    text: Color(0xFF12191C),
    muted: Color(0xFF566268),
    border: Color(0xFFD5DEE0),
    accent: Color(0xFF0E7D88),
    accentContrast: Color(0xFFFFFFFF),
    ok: Color(0xFF1D7A46),
    warn: Color(0xFF8F6000),
    danger: Color(0xFFC0392B),
    okBg: Color(0xFFE4F3EA),
    warnBg: Color(0xFFFBF0DD),
    dangerBg: Color(0xFFFBE7E4),
    artBg: Color(0xFF0C1113),
    isDark: false,
  );

  static const Tokens retro = Tokens(
    bg: Color(0xFF221913),
    surface: Color(0xFF30241A),
    surfaceAlt: Color(0xFF3D2F21),
    text: Color(0xFFF4E7D1),
    muted: Color(0xFFB9A285),
    border: Color(0xFF4D3C2A),
    accent: Color(0xFFE0913A),
    accentContrast: Color(0xFF241206),
    ok: Color(0xFF8FAA46),
    warn: Color(0xFFE8C749),
    danger: Color(0xFFEC7C5E),
    okBg: Color(0xFF29301A),
    warnBg: Color(0xFF37311A),
    dangerBg: Color(0xFF341C14),
    artBg: Color(0xFF140E08),
    isDark: true,
  );

  static Tokens of(AppThemeVariant variant, {bool highContrast = false}) {
    final Tokens base = switch (variant) {
      AppThemeVariant.dark => dark,
      AppThemeVariant.light => light,
      AppThemeVariant.retro => retro,
    };
    return highContrast ? base.hardened() : base;
  }

  Tokens hardened() => Tokens(
    bg: bg,
    surface: surface,
    surfaceAlt: surfaceAlt,
    text: text,
    muted: text,
    border: text,
    accent: accent,
    accentContrast: accentContrast,
    ok: ok,
    warn: warn,
    danger: danger,
    okBg: okBg,
    warnBg: warnBg,
    dangerBg: dangerBg,
    artBg: artBg,
    isDark: isDark,
    rowTintAlpha: 0.3,
    rowHoverAlpha: 0.18,
  );

  Color get rowOk => ok.withValues(alpha: rowTintAlpha);
  Color get rowWarn => warn.withValues(alpha: rowTintAlpha);
  Color get rowDanger => danger.withValues(alpha: rowTintAlpha);
  Color get rowHover => text.withValues(alpha: rowHoverAlpha);

  Color get fieldBorder => Color.lerp(border, muted, 0.7)!;
}
