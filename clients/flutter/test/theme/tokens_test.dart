import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

void main() {
  test('hologram is blue line work on a navy ground', () {
    const Tokens t = Tokens.hologram;

    expect(t.bg, const Color(0xFF05080D));
    expect(t.surface, const Color(0xFF0A1621));
    expect(t.surfaceAlt, const Color(0xFF10202F));
    expect(t.text, const Color(0xFFE3F3FF));
    expect(t.muted, const Color(0xFF82A6C0));
    expect(t.border, const Color(0xFF1B3449));
    expect(t.accent, const Color(0xFF45B8FF));
    expect(t.accentContrast, const Color(0xFF03111C));
    expect(t.ok, const Color(0xFF3FE0A5));
    expect(t.warn, const Color(0xFFFFB547));
    expect(t.danger, const Color(0xFFFF4D5E));
    expect(t.okBg, const Color(0xFF0A2620));
    expect(t.warnBg, const Color(0xFF2B2010));
    expect(t.dangerBg, const Color(0xFF2C0E15));
    expect(t.artBg, const Color(0xFF020407));
    expect(t.isDark, isTrue);
  });

  test('the hologram variant picks the hologram tokens', () {
    expect(Tokens.of(AppThemeVariant.hologram), same(Tokens.hologram));
  });

  test('high contrast hardens hologram as it does the other themes', () {
    final Tokens hard = Tokens.of(AppThemeVariant.hologram, highContrast: true);

    expect(hard.muted, Tokens.hologram.text);
    expect(hard.border, Tokens.hologram.text);
    expect(hard.accent, Tokens.hologram.accent);
    expect(hard.bg, Tokens.hologram.bg);
  });

  for (final AppThemeVariant variant in <AppThemeVariant>[
    AppThemeVariant.dark,
    AppThemeVariant.light,
    AppThemeVariant.retro,
  ]) {
    test('${variant.name} lays its texture in the text colour at 6%', () {
      final Tokens t = Tokens.of(variant);

      expect(t.texture, t.text.withValues(alpha: 0.06));
    });
  }

  test('hologram lays its texture in the accent at 11%', () {
    expect(
      Tokens.hologram.texture,
      Tokens.hologram.accent.withValues(alpha: 0.11),
    );
  });

  test('high contrast keeps every theme\'s texture', () {
    for (final AppThemeVariant variant in AppThemeVariant.values) {
      expect(
        Tokens.of(variant, highContrast: true).texture,
        Tokens.of(variant).texture,
        reason: variant.name,
      );
    }
  });
}
