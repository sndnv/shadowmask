import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/brand_mark.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Future<void> _pump(WidgetTester tester, AppThemeVariant variant) async {
  await tester.pumpWidget(
    ThemeScope(
      variant: variant,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(variant),
        home: const Scaffold(body: Center(child: BrandMark())),
      ),
    ),
  );
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 50));
}

void main() {
  testWidgets('renders an svg brand mark', (WidgetTester tester) async {
    await _pump(tester, AppThemeVariant.dark);
    expect(find.byType(BrandMark), findsOneWidget);
    expect(find.byType(SvgPicture), findsOneWidget);
  });

  testWidgets('renders in the retro variant', (WidgetTester tester) async {
    await _pump(tester, AppThemeVariant.retro);
    expect(find.byType(SvgPicture), findsOneWidget);
  });

  testWidgets('exposes the app name as its semantic label', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle handle = tester.ensureSemantics();
    await _pump(tester, AppThemeVariant.dark);
    expect(find.bySemanticsLabel(Strings.appName), findsOneWidget);
    handle.dispose();
  });

  test('the dots are drawn back from full strength', () {
    expect(
      brandMarkSvg('#35C9D6', retro: false),
      contains('fill-opacity="$kBrandDotOpacity"'),
    );
    expect(kBrandDotOpacity, isNot('1'));
  });

  test('a plain mark paints every dot in the accent', () {
    final String svg = brandMarkSvg('#35C9D6', retro: false);

    expect('#35C9D6'.allMatches(svg).length, 26);
    for (final String phosphor in kPhosphor) {
      expect(svg, isNot(contains(phosphor)));
    }
  });

  test('the retro mark staggers each row into a delta triad', () {
    final List<String> fills = _fills(brandMarkSvg('#35C9D6', retro: true));

    expect(fills.length, 25);
    for (int row = 0; row < 5; row++) {
      for (int col = 0; col < 5; col++) {
        expect(fills[row * 5 + col], kPhosphor[(col + row) % 3]);
      }
    }
  });

  test('no retro column is a single solid colour', () {
    final List<String> fills = _fills(brandMarkSvg('#35C9D6', retro: true));

    for (int col = 0; col < 5; col++) {
      final Set<String> column = <String>{
        for (int row = 0; row < 5; row++) fills[row * 5 + col],
      };
      expect(column.length, greaterThan(1));
    }
  });
}

List<String> _fills(String svg) => RegExp(
  r'<circle[^>]*fill="([^"]+)"',
).allMatches(svg).map((RegExpMatch m) => m.group(1)!).toList();
