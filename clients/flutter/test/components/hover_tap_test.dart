import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

Widget _host({
  required VoidCallback onTap,
  String? semanticsLabel,
  Widget child = const Text('Blade Runner'),
}) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(
    body: Center(
      child: HoverTap(
        onTap: onTap,
        semanticsLabel: semanticsLabel,
        child: child,
      ),
    ),
  ),
);

Color? _ringColor(WidgetTester tester) {
  final BoxDecoration decoration =
      tester
              .widget<DecoratedBox>(
                find.descendant(
                  of: find.byType(HoverTap),
                  matching: find.byType(DecoratedBox),
                ),
              )
              .decoration
          as BoxDecoration;
  return decoration.border?.top.color;
}

void main() {
  testWidgets('tab reaches it and Enter opens it', (WidgetTester tester) async {
    int taps = 0;
    await tester.pumpWidget(_host(onTap: () => taps++));

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pump();

    expect(taps, 1);
  });

  testWidgets('the space bar opens it too', (WidgetTester tester) async {
    int taps = 0;
    await tester.pumpWidget(_host(onTap: () => taps++));

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pump();

    expect(taps, 1);
  });

  testWidgets('the ring shows only once focus is on it', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(onTap: () {}));

    // Transparent rather than absent, so arriving on it moves no pixels.
    expect(_ringColor(tester), const Color(0x00000000));

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();

    expect(_ringColor(tester), Tokens.dark.accent);
  });

  testWidgets('its own content names it', (WidgetTester tester) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await tester.pumpWidget(_host(onTap: () {}));

    expect(
      tester.getSemantics(find.byType(HoverTap)),
      matchesSemantics(
        label: 'Blade Runner',
        isButton: true,
        isFocusable: true,
        hasTapAction: true,
        hasFocusAction: true,
      ),
    );

    semantics.dispose();
  });

  testWidgets('a label names a control that has no text of its own', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await tester.pumpWidget(
      _host(
        onTap: () {},
        semanticsLabel: 'Back',
        child: const Icon(Icons.arrow_back),
      ),
    );

    expect(tester.getSemantics(find.byType(HoverTap)).label, 'Back');

    semantics.dispose();
  });

  testWidgets('a pointer tap still works', (WidgetTester tester) async {
    int taps = 0;
    await tester.pumpWidget(_host(onTap: () => taps++));

    await tester.tap(find.text('Blade Runner'));
    await tester.pump();

    expect(taps, 1);
  });
}
