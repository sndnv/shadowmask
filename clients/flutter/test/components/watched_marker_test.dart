import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/watched_marker.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/tokens.dart';

Future<void> _pump(WidgetTester tester, AppThemeVariant variant) =>
    tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(variant),
        home: const Scaffold(body: Center(child: WatchedMarker())),
      ),
    );

Finder get _plate => find.descendant(
  of: find.byType(WatchedMarker),
  matching: find.byType(Container),
);

Icon _check(WidgetTester tester) =>
    tester.widget<Icon>(find.byIcon(Icons.check));

void main() {
  testWidgets('hologram marks watched with the mock\'s hexagon', (
    WidgetTester tester,
  ) async {
    await _pump(tester, AppThemeVariant.hologram);
    const Tokens t = Tokens.hologram;

    expect(tester.getSize(_plate), Marks.watched);
    expect(
      tester.widget<Container>(_plate).decoration,
      ShapeDecoration(color: t.ok, shape: Shapes.hexagon(Marks.watched)),
    );
    expect(_check(tester).size, 16);
    expect(_check(tester).color, t.accentContrast);
  });

  for (final AppThemeVariant variant in AppThemeVariant.values.where(
    (AppThemeVariant v) => v != AppThemeVariant.hologram,
  )) {
    testWidgets('${variant.name} keeps the ringed disc', (
      WidgetTester tester,
    ) async {
      await _pump(tester, variant);
      final Tokens t = Tokens.of(variant);

      expect(tester.getSize(_plate), const Size(22, 22));
      expect(
        tester.widget<Container>(_plate).decoration,
        BoxDecoration(
          color: t.ok,
          shape: BoxShape.circle,
          border: Border.fromBorderSide(BorderSide(color: t.surface, width: 2)),
        ),
      );
      expect(_check(tester).size, 13);
      expect(_check(tester).color, Colors.white);
    });
  }
}
