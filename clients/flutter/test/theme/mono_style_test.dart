import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

void main() {
  test('figures line up without leaving the body font', () {
    expect(monoStyle.fontFamily, isNull);
    expect(monoStyle.fontFamilyFallback, isNull);
    expect(monoStyle.fontFeatures, <FontFeature>[
      const FontFeature.tabularFigures(),
    ]);
  });

  testWidgets('a timecode keeps the theme font and its aligned digits', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: Text(
            '1:02:33',
            style: monoStyle.copyWith(fontWeight: FontWeight.w700),
          ),
        ),
      ),
    );

    final TextStyle body = Theme.of(
      tester.element(find.text('1:02:33')),
    ).textTheme.bodyMedium!;
    final TextStyle drawn = tester
        .widget<RichText>(find.byType(RichText))
        .text
        .style!;

    expect(drawn.fontFamily, body.fontFamily);
    expect(drawn.fontFeatures, contains(const FontFeature.tabularFigures()));
    expect(drawn.fontWeight, FontWeight.w700);
  });
}
