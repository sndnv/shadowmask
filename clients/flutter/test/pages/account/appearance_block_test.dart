import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/appearance_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';

Widget _host({
  required AppThemeVariant variant,
  required ValueChanged<AppThemeVariant> onSet,
  double? width,
  bool highContrast = false,
  ValueChanged<bool>? onContrast,
}) {
  return ThemeScope(
    variant: variant,
    setVariant: onSet,
    highContrast: highContrast,
    setHighContrast: onContrast ?? (_) {},
    child: MaterialApp(
      theme: buildTheme(variant, highContrast: highContrast),
      home: Scaffold(
        // the account page scrolls, so the block is never height-constrained
        body: SingleChildScrollView(
          child: SizedBox(width: width, child: const AppearanceBlock()),
        ),
      ),
    ),
  );
}

double _optionWidth(WidgetTester tester, String label) => tester
    .getSize(
      find
          .ancestor(of: find.text(label), matching: find.byType(Container))
          .first,
    )
    .width;

void main() {
  testWidgets('shows all three themes as choices', (WidgetTester tester) async {
    await tester.pumpWidget(
      _host(variant: AppThemeVariant.dark, onSet: (_) {}),
    );

    expect(find.text(Strings.themeDark), findsOneWidget);
    expect(find.text(Strings.themeLight), findsOneWidget);
    expect(find.text(Strings.themeRetro), findsOneWidget);
  });

  testWidgets('tapping a theme sets that variant', (WidgetTester tester) async {
    AppThemeVariant? picked;
    await tester.pumpWidget(
      _host(
        variant: AppThemeVariant.dark,
        onSet: (AppThemeVariant v) => picked = v,
      ),
    );

    await tester.tap(find.text(Strings.themeLight));
    await tester.pump();

    expect(picked, AppThemeVariant.light);
  });

  testWidgets('a wide block keeps the fixed option width', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(variant: AppThemeVariant.dark, onSet: (_) {}, width: 700),
    );

    expect(_optionWidth(tester, Strings.themeDark), kThemeOptionWidth);
  });

  testWidgets('a phone gives each option the whole row', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(variant: AppThemeVariant.dark, onSet: (_) {}, width: 304),
    );

    // At 168 of the row the block looked half finished. The row is the
    // section's inner width, so it is narrower than the 304 body.
    final double row = tester
        .getSize(
          find
              .ancestor(
                of: find.text(Strings.themeDark),
                matching: find.byType(Wrap),
              )
              .first,
        )
        .width;
    expect(_optionWidth(tester, Strings.themeDark), row);
    expect(row, lessThan(304));
    expect(
      tester.getTopLeft(find.text(Strings.themeLight)).dy,
      greaterThan(tester.getBottomLeft(find.text(Strings.themeDark)).dy),
    );
  });

  testWidgets('the high contrast switch reads the scope', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(variant: AppThemeVariant.dark, onSet: (_) {}, highContrast: true),
    );

    expect(find.text(Strings.highContrast), findsOneWidget);
    expect(
      tester.widget<Switch>(find.byType(Switch)).value,
      isTrue,
      reason: 'the switch has no state of its own; the scope owns it',
    );
  });

  testWidgets('toggling high contrast reports the new value', (
    WidgetTester tester,
  ) async {
    bool? picked;
    await tester.pumpWidget(
      _host(
        variant: AppThemeVariant.dark,
        onSet: (_) {},
        onContrast: (bool v) => picked = v,
      ),
    );

    await tester.tap(find.byType(Switch));
    await tester.pump();

    expect(picked, isTrue);
  });

  testWidgets('the previews harden with the rest of the app', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(variant: AppThemeVariant.dark, onSet: (_) {}, highContrast: true),
    );

    final Container preview = tester.widget<Container>(
      find
          .ancestor(
            of: find.text(Strings.themeLight),
            matching: find.byType(Container),
          )
          .first,
    );
    final BoxDecoration decoration = preview.decoration! as BoxDecoration;

    expect(decoration.color, Tokens.dark.hardened().surfaceAlt);
  });
}
