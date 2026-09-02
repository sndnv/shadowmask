import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/danger_icon_button.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

Future<void> _pump(
  WidgetTester tester,
  IconData icon,
  VoidCallback? onPressed,
) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: DangerIconButton(
          icon: icon,
          tooltip: 'Do the thing',
          onPressed: onPressed,
        ),
      ),
    ),
  );
}

void main() {
  testWidgets('paints the icon in the error colour', (
    WidgetTester tester,
  ) async {
    await _pump(tester, Icons.delete_outline, () {});

    final Color painted = tester
        .widget<IconTheme>(
          find
              .ancestor(
                of: find.byIcon(Icons.delete_outline),
                matching: find.byType(IconTheme),
              )
              .first,
        )
        .data
        .color!;
    expect(painted, Tokens.dark.danger);
  });

  testWidgets('carries the tooltip and reports a press', (
    WidgetTester tester,
  ) async {
    bool pressed = false;
    await _pump(tester, Icons.cancel_outlined, () => pressed = true);

    expect(find.byTooltip('Do the thing'), findsOneWidget);
    await tester.tap(find.byIcon(Icons.cancel_outlined));
    expect(pressed, isTrue);
  });
}
