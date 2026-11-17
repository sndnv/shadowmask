import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_ext.dart';
import 'package:shadowmask/theme/tokens.dart';

const List<AppThemeVariant> _rounded = <AppThemeVariant>[
  AppThemeVariant.dark,
  AppThemeVariant.light,
  AppThemeVariant.retro,
];

BorderRadius _bevel(double cut) => BorderRadius.only(
  topLeft: Radius.circular(cut),
  bottomRight: Radius.circular(cut),
);

OutlinedBorder? _shapeOf(ButtonStyle? style) =>
    style?.shape?.resolve(<WidgetState>{});

final List<BoxShadow> _tooltipShadows = <BoxShadow>[
  BoxShadow(
    color: Colors.black.withValues(alpha: 0.35),
    blurRadius: 12,
    offset: const Offset(0, 4),
  ),
];

BoxDecoration _tooltip(AppThemeVariant variant) {
  final Tokens t = Tokens.of(variant);
  return BoxDecoration(
    color: t.surfaceAlt,
    borderRadius: const BorderRadius.all(Radii.sm),
    border: Border.all(color: t.border),
    boxShadow: _tooltipShadows,
  );
}

void main() {
  test('every theme carries its shape set', () {
    for (final AppThemeVariant variant in AppThemeVariant.values) {
      final ShapesExt? ext = buildTheme(variant).extension<ShapesExt>();
      expect(ext, isNotNull, reason: variant.name);
      expect(
        ext!.shapes.cut,
        variant == AppThemeVariant.hologram,
        reason: variant.name,
      );
    }
  });

  for (final AppThemeVariant variant in _rounded) {
    test('${variant.name} keeps its rounded theme shapes', () {
      final ThemeData theme = buildTheme(variant);
      final Tokens t = Tokens.of(variant);
      const RoundedRectangleBorder button = RoundedRectangleBorder(
        borderRadius: BorderRadius.all(Radii.sm),
      );

      expect(_shapeOf(theme.filledButtonTheme.style), button);
      expect(_shapeOf(theme.textButtonTheme.style), button);
      expect(_shapeOf(theme.outlinedButtonTheme.style), button);
      expect(
        theme.outlinedButtonTheme.style!.side!.resolve(<WidgetState>{}),
        BorderSide(color: t.border),
      );
      expect(
        theme.dialogTheme.shape,
        const RoundedRectangleBorder(borderRadius: BorderRadius.all(Radii.md)),
      );
      expect(
        theme.popupMenuTheme.shape,
        RoundedRectangleBorder(
          borderRadius: const BorderRadius.all(Radii.md),
          side: BorderSide(color: t.border),
        ),
      );
      expect(theme.tooltipTheme.decoration, _tooltip(variant));
      expect(
        theme.inputDecorationTheme.border,
        const OutlineInputBorder(borderRadius: BorderRadius.all(Radii.sm)),
      );
      expect(
        theme.inputDecorationTheme.enabledBorder,
        OutlineInputBorder(
          borderRadius: const BorderRadius.all(Radii.sm),
          borderSide: BorderSide(color: t.fieldBorder),
        ),
      );
      expect(
        theme.inputDecorationTheme.focusedBorder,
        OutlineInputBorder(
          borderRadius: const BorderRadius.all(Radii.sm),
          borderSide: BorderSide(color: t.accent, width: 2),
        ),
      );
      expect(
        theme.checkboxTheme.shape,
        const RoundedRectangleBorder(
          borderRadius: BorderRadius.all(Radius.circular(3)),
        ),
      );
      expect(theme.iconButtonTheme.style, isNull);
      expect(theme.bottomSheetTheme.shape, isNull);
      expect(theme.chipTheme.shape, isNull);
    });
  }

  test('hologram cuts its theme shapes', () {
    final ThemeData theme = buildTheme(AppThemeVariant.hologram);
    final CutBorder button = CutBorder(borderRadius: _bevel(8));

    expect(_shapeOf(theme.filledButtonTheme.style), button);
    expect(_shapeOf(theme.textButtonTheme.style), button);
    expect(_shapeOf(theme.outlinedButtonTheme.style), button);
    expect(theme.dialogTheme.shape, CutBorder(borderRadius: _bevel(14)));
    final BorderSide line = BorderSide(
      color: Tokens.hologram.accent.withValues(alpha: 0.62),
    );
    expect(
      theme.popupMenuTheme.shape,
      CutBorder(borderRadius: _bevel(14), side: line),
    );
    expect(
      theme.tooltipTheme.decoration,
      ShapeDecoration(
        color: Tokens.hologram.surfaceAlt,
        shadows: _tooltipShadows,
        shape: CutBorder(borderRadius: _bevel(6), side: line),
      ),
    );
    expect(theme.chipTheme.shape, CutBorder(borderRadius: _bevel(6)));
    expect(
      theme.bottomSheetTheme.shape,
      const CutBorder(
        borderRadius: BorderRadius.only(topLeft: Radius.circular(14)),
      ),
    );
  });

  test('hologram squares its fields and checkboxes', () {
    final ThemeData theme = buildTheme(AppThemeVariant.hologram);
    final InputDecorationThemeData fields = theme.inputDecorationTheme;

    for (final InputBorder? border in <InputBorder?>[
      fields.border,
      fields.enabledBorder,
      fields.focusedBorder,
    ]) {
      expect((border! as OutlineInputBorder).borderRadius, BorderRadius.zero);
    }
    expect(
      theme.checkboxTheme.shape,
      CutBorder(borderRadius: BorderRadius.zero),
    );
  });

  test('hologram turns the icon buttons\' highlight into a hexagon', () {
    final ThemeData theme = buildTheme(AppThemeVariant.hologram);

    expect(
      _shapeOf(theme.iconButtonTheme.style),
      Shapes.cut(Tokens.hologram, highContrast: false).round(),
    );
  });

  test(
    'hologram outlines its menus and outlined buttons in the line colour',
    () {
      final ThemeData theme = buildTheme(AppThemeVariant.hologram);
      final Color line = Tokens.hologram.accent.withValues(alpha: 0.62);

      expect(
        (theme.popupMenuTheme.shape! as BeveledRectangleBorder).side.color,
        line,
      );
      expect(
        theme.outlinedButtonTheme.style!.side!.resolve(<WidgetState>{})!.color,
        line,
      );
    },
  );
}
