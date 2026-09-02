import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

void main() {
  for (final AppThemeVariant variant in AppThemeVariant.values) {
    test('${variant.name} tooltips are drawn from the Projection palette', () {
      final Tokens t = Tokens.of(variant);
      final TooltipThemeData tooltip = buildTheme(variant).tooltipTheme;
      final BoxDecoration decoration = tooltip.decoration! as BoxDecoration;

      expect(decoration.color, t.surfaceAlt);
      expect(decoration.border, Border.all(color: t.border));
      expect(tooltip.textStyle?.color, t.text);
    });
  }

  testWidgets('a tooltip picks the theme up without any per-site styling', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: const Scaffold(
          body: Tooltip(message: 'Delete library', child: Icon(Icons.delete)),
        ),
      ),
    );

    final Tooltip tooltip = tester.widget<Tooltip>(find.byType(Tooltip));
    expect(tooltip.decoration, isNull);
    expect(tooltip.textStyle, isNull);
  });
}
