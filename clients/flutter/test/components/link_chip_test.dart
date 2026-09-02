import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/link_chip.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

void main() {
  Future<void> pumpChip(WidgetTester tester, {VoidCallback? onTap}) =>
      tester.pumpWidget(
        MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: Scaffold(
            body: Center(
              child: LinkChip(label: 'Drama', onTap: onTap),
            ),
          ),
        ),
      );

  Material materialOf(WidgetTester tester) => tester.widget<Material>(
    find.descendant(of: find.byType(LinkChip), matching: find.byType(Material)),
  );

  Color borderOf(WidgetTester tester) =>
      (materialOf(tester).shape! as RoundedRectangleBorder).side.color;

  Color? fillOf(WidgetTester tester) => materialOf(tester).color;

  Future<void> hover(WidgetTester tester) async {
    final TestGesture pointer = await tester.createGesture(
      kind: PointerDeviceKind.mouse,
    );
    await pointer.addPointer(location: Offset.zero);
    addTearDown(pointer.removePointer);
    await tester.pump();
    await pointer.moveTo(tester.getCenter(find.byType(LinkChip)));
    await tester.pumpAndSettle();
  }

  testWidgets('a chip that links rests on the accent border', (
    WidgetTester tester,
  ) async {
    await pumpChip(tester, onTap: () {});

    expect(borderOf(tester), Tokens.dark.accent);
    expect(fillOf(tester), Tokens.dark.surfaceAlt);
    expect(
      tester.widget<Text>(find.text('Drama')).style?.color,
      Tokens.dark.text,
    );
  });

  testWidgets('hover is carried by the fill and the text, not the border', (
    WidgetTester tester,
  ) async {
    await pumpChip(tester, onTap: () {});

    await hover(tester);

    expect(borderOf(tester), Tokens.dark.accent);
    expect(fillOf(tester), Tokens.dark.accent.withValues(alpha: 0.12));
    expect(
      tester.widget<Text>(find.text('Drama')).style?.color,
      Tokens.dark.accent,
    );
  });

  testWidgets('a chip with no destination keeps the plain border', (
    WidgetTester tester,
  ) async {
    await pumpChip(tester);

    expect(borderOf(tester), Tokens.dark.border);

    await hover(tester);

    expect(borderOf(tester), Tokens.dark.border);
    expect(
      tester.widget<Text>(find.text('Drama')).style?.color,
      Tokens.dark.text,
    );
  });
}
