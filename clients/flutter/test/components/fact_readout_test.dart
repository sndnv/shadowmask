import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/fact.dart';
import 'package:shadowmask/components/fact_readout.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/tokens.dart';

Future<void> _pump(WidgetTester tester, {bool highContrast = false}) =>
    tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.hologram, highContrast: highContrast),
        home: const Scaffold(
          body: Center(child: FactReadout(Fact(Strings.factYear, '2019'))),
        ),
      ),
    );

Iterable<ShapeDecoration> _hexagons(WidgetTester tester) => tester
    .widgetList<DecoratedBox>(
      find.descendant(
        of: find.byType(FactReadout),
        matching: find.byType(DecoratedBox),
      ),
    )
    .map((DecoratedBox box) => box.decoration)
    .whereType<ShapeDecoration>();

void main() {
  testWidgets('the value sits in a small hexagon outlined in the line', (
    WidgetTester tester,
  ) async {
    await _pump(tester);
    const Tokens t = Tokens.hologram;
    final ShapeDecoration outline = _hexagons(tester).first;
    final ShapeDecoration face = _hexagons(tester).elementAt(1);
    final Finder value = find.text('2019');

    expect(outline.color, t.accent.withValues(alpha: 0.62));
    expect(outline.shape, Shapes.hexagon(Marks.readout));
    expect(face.color, t.surface);
    final Finder boxes = find.descendant(
      of: find.byType(FactReadout),
      matching: find.byType(DecoratedBox),
    );
    expect(tester.getSize(boxes.at(0)), Marks.readout);
    expect(Marks.readout, const Size(44, 38));
    expect(
      tester.getSize(boxes.at(1)),
      Marks.readout * (1 - 2.4 / Marks.readout.height),
    );
    expect(find.descendant(of: boxes.at(1), matching: value), findsOneWidget);
    expect(tester.widget<Text>(value).style?.fontSize, 13);
    expect(tester.widget<Text>(value).style?.fontWeight, FontWeight.w700);
  });

  testWidgets('the hexagon grows with the text so the value still fits', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.hologram),
        home: const MediaQuery(
          data: MediaQueryData(textScaler: TextScaler.linear(1.5)),
          child: Scaffold(
            body: Center(child: FactReadout(Fact(Strings.factYear, '2019'))),
          ),
        ),
      ),
    );

    final Rect value = tester.getRect(find.text('2019'));
    final Rect hexagon = tester.getRect(
      find
          .descendant(
            of: find.byType(FactReadout),
            matching: find.byType(DecoratedBox),
          )
          .first,
    );
    expect(hexagon.size, Marks.readout * 1.5);
    expect(hexagon.contains(value.topLeft), isTrue);
    expect(hexagon.contains(value.bottomRight), isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets('the dot beside the line is an accent hexagon', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    final ShapeDecoration dot = _hexagons(tester).elementAt(2);
    expect(dot.shape, Shapes.hexagon(Marks.readoutDot));
    expect(dot.color, Tokens.hologram.accent);
  });

  testWidgets('high contrast draws the dot in the text colour so it stands '
      'apart from the solid line', (WidgetTester tester) async {
    await _pump(tester, highContrast: true);

    final Color line = tester
        .widgetList<Container>(
          find.descendant(
            of: find.byType(FactReadout),
            matching: find.byType(Container),
          ),
        )
        .singleWhere((Container c) => c.constraints?.maxHeight == 1)
        .color!;
    final ShapeDecoration dot = _hexagons(tester).elementAt(2);
    expect(line, Tokens.hologram.accent);
    expect(dot.shape, Shapes.hexagon(Marks.readoutDot));
    expect(dot.color, Tokens.hologram.text);
  });

  testWidgets('the label is the unit, in capitals but read as written', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    final Text unit = tester.widget<Text>(find.text('YEAR'));
    expect(unit.semanticsLabel, Strings.factYear);
    expect(unit.style?.fontFamily, kMonoFamily);
    expect(unit.style?.color, Tokens.hologram.accent);
    expect(
      tester.getCenter(find.text('YEAR')).dx,
      greaterThan(tester.getRect(find.text('2019')).right),
    );
  });
}
