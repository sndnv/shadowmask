import 'dart:math' as math;

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadowmask/components/edged_box.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';

import '../support/raster.dart';

const Key _frame = ValueKey<String>('frame');
const Key _fill = ValueKey<String>('fill');
const Color _ground = Color(0xFF102030);
const Color _content = Color(0xFF305070);
const Color _ink = Color(0xFFE0F0FF);
const BorderSide _edge = BorderSide(color: _ink);

Future<void> _pump(
  WidgetTester tester, {
  AppThemeVariant variant = AppThemeVariant.hologram,
  bool inset = true,
  BorderSide edge = _edge,
  Clip clipBehavior = Clip.antiAlias,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(variant),
      home: Align(
        alignment: Alignment.topLeft,
        child: RepaintBoundary(
          key: _frame,
          child: EdgedBox(
            corner: Corners.panel,
            color: _ground,
            edge: edge,
            width: 120,
            height: 80,
            inset: inset,
            clipBehavior: clipBehavior,
            child: const ColoredBox(key: _fill, color: _content),
          ),
        ),
      ),
    ),
  );
}

Offset _cut(double inward) {
  final double step = inward / math.sqrt2;
  return Offset(7 + step, 7 + step);
}

Offset _farCut(double inward) {
  final double step = inward / math.sqrt2;
  return Offset(113 - step, 73 - step);
}

Container _box(WidgetTester tester) => tester.widget<Container>(
  find.descendant(of: find.byType(EdgedBox), matching: find.byType(Container)),
);

void main() {
  testWidgets('a hologram edge runs over content that reaches the cut', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    final List<Color> pixels = await pixelsAt(
      tester,
      find.byKey(_frame),
      <Offset>[_cut(0.5), _farCut(0.5), const Offset(0.5, 40), _cut(1.5)],
    );

    expect(pixels, <Matcher>[
      isPixel(_ink),
      isPixel(_ink),
      isPixel(_ink),
      isPixel(_content),
    ]);
  });

  testWidgets('a hologram box keeps its content inside the edge', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(tester.getTopLeft(find.byKey(_fill)), const Offset(1, 1));
    expect(tester.getSize(find.byKey(_fill)), const Size(118, 78));
  });

  for (final AppThemeVariant variant in <AppThemeVariant>[
    AppThemeVariant.dark,
    AppThemeVariant.hologram,
  ]) {
    testWidgets('a ${variant.name} box insets by the part of the edge inside '
        'it', (WidgetTester tester) async {
      await _pump(
        tester,
        variant: variant,
        edge: const BorderSide(
          color: _ink,
          width: 2,
          strokeAlign: BorderSide.strokeAlignCenter,
        ),
      );

      expect(tester.getTopLeft(find.byKey(_fill)), const Offset(1, 1));
    });
  }

  testWidgets('a box can leave its content unclipped', (
    WidgetTester tester,
  ) async {
    await _pump(tester, clipBehavior: Clip.none);

    expect(_box(tester).clipBehavior, Clip.none);
  });

  testWidgets('a hologram box without an inset runs its content to the edge', (
    WidgetTester tester,
  ) async {
    await _pump(tester, inset: false);

    expect(tester.getTopLeft(find.byKey(_fill)), Offset.zero);
    expect(tester.getSize(find.byKey(_fill)), const Size(120, 80));
  });

  testWidgets('a rounded box keeps its border under the content', (
    WidgetTester tester,
  ) async {
    await _pump(tester, variant: AppThemeVariant.dark);

    final Container box = _box(tester);

    expect(box.foregroundDecoration, isNull);
    expect(
      box.decoration,
      const Shapes.rounded().box(Corners.panel, color: _ground, side: _edge),
    );
    expect(box.clipBehavior, Clip.antiAlias);
    expect(tester.getTopLeft(find.byKey(_fill)), const Offset(1, 1));
  });
}
