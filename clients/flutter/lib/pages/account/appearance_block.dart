import 'package:flutter/material.dart';

import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kThemeOptionWidth = 168;

class AppearanceBlock extends StatelessWidget {
  const AppearanceBlock({super.key});

  static String _label(AppThemeVariant v) => switch (v) {
    AppThemeVariant.dark => Strings.themeDark,
    AppThemeVariant.light => Strings.themeLight,
    AppThemeVariant.retro => Strings.themeRetro,
  };

  @override
  Widget build(BuildContext context) {
    final ThemeScope scope = ThemeScope.of(context);
    final Tokens t = context.tokens;
    return SectionBlock(
      title: Strings.appearanceHeading,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          LayoutBuilder(
            builder: (BuildContext context, BoxConstraints constraints) => Wrap(
              spacing: Space.s3,
              runSpacing: Space.s3,
              children: <Widget>[
                for (final AppThemeVariant v in AppThemeVariant.values)
                  _ThemeOption(
                    label: _label(v),
                    preview: Tokens.of(v, highContrast: scope.highContrast),
                    selected: v == scope.variant,
                    width: constraints.maxWidth < Breakpoints.sm
                        ? constraints.maxWidth
                        : kThemeOptionWidth,
                    onTap: () => scope.setVariant(v),
                  ),
              ],
            ),
          ),
          const SizedBox(height: Space.s3),
          Divider(color: t.border, height: 1),
          Material(
            color: Colors.transparent,
            child: SwitchListTile(
              contentPadding: EdgeInsets.zero,
              value: scope.highContrast,
              onChanged: scope.setHighContrast,
              title: const Text(
                Strings.highContrast,
                style: TextStyle(fontWeight: FontWeight.w600),
              ),
              subtitle: Text(
                Strings.highContrastHelp,
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: t.muted),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ThemeOption extends StatelessWidget {
  const _ThemeOption({
    required this.label,
    required this.preview,
    required this.selected,
    required this.width,
    required this.onTap,
  });

  final String label;
  final Tokens preview;
  final bool selected;
  final double width;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return InkWell(
      onTap: onTap,
      borderRadius: const BorderRadius.all(Radii.md),
      child: Container(
        width: width,
        padding: const EdgeInsets.all(Space.s2),
        decoration: BoxDecoration(
          color: t.surfaceAlt,
          borderRadius: const BorderRadius.all(Radii.md),
          border: Border.all(
            color: selected ? t.accent : t.border,
            width: selected ? 2 : 1,
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Container(
              height: 44,
              padding: const EdgeInsets.all(Space.s2),
              decoration: BoxDecoration(
                color: preview.bg,
                borderRadius: const BorderRadius.all(Radii.sm),
                border: Border.all(color: preview.border),
              ),
              child: Row(
                children: <Widget>[
                  _swatch(preview.surface, preview.border),
                  const SizedBox(width: Space.s2),
                  _swatch(preview.accent, preview.border),
                  const SizedBox(width: Space.s2),
                  _swatch(preview.text, preview.border),
                ],
              ),
            ),
            const SizedBox(height: Space.s2),
            Row(
              children: <Widget>[
                Text(
                  label,
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
                const Spacer(),
                if (selected)
                  Icon(Icons.check_circle, size: 18, color: t.accent),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _swatch(Color color, Color border) => Container(
    width: 20,
    height: 20,
    decoration: BoxDecoration(
      color: color,
      shape: BoxShape.circle,
      border: Border.all(color: border),
    ),
  );
}
