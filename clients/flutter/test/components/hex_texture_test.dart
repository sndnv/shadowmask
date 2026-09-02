import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/hex_texture.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

void main() {
  testWidgets('paints and ignores pointer events', (WidgetTester tester) async {
    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: const Scaffold(body: HexTexture()),
        ),
      ),
    );
    await tester.pump();

    expect(find.byType(HexTexture), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(HexTexture),
        matching: find.byType(IgnorePointer),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byType(HexTexture),
        matching: find.byType(CustomPaint),
      ),
      findsOneWidget,
    );
  });
}
