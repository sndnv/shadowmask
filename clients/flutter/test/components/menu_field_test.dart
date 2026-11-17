import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';

import '../support/raster.dart';

Future<void> _pump(
  WidgetTester tester,
  AppThemeVariant variant,
  Widget child,
) => tester.pumpWidget(
  MaterialApp(
    theme: buildTheme(variant),
    home: Scaffold(body: Center(child: child)),
  ),
);

const Key _frame = Key('frame');

Widget _field({required bool enabled}) => RepaintBoundary(
  key: _frame,
  child: MenuField(
    open: false,
    enabled: enabled,
    onTap: () {},
    child: const Text('Added'),
  ),
);

Material _material(WidgetTester tester) => tester.widget<Material>(
  find.descendant(of: find.byType(MenuField), matching: find.byType(Material)),
);

Future<Color> _paddingPixel(WidgetTester tester) async {
  final Size field = tester.getSize(find.byType(MenuField));
  final List<Color> pixels = await pixelsAt(
    tester,
    find.byKey(_frame),
    <Offset>[Offset(Space.s1, field.height / 2)],
  );
  return pixels.single;
}

Future<void> _hover(WidgetTester tester) async {
  final TestGesture pointer = await tester.createGesture(
    kind: PointerDeviceKind.mouse,
  );
  await pointer.addPointer(location: Offset.zero);
  addTearDown(pointer.removePointer);
  await tester.pump();
  await pointer.moveTo(tester.getCenter(find.byType(MenuField)));
  await tester.pumpAndSettle();
}

void main() {
  for (final AppThemeVariant variant in <AppThemeVariant>[
    AppThemeVariant.dark,
    AppThemeVariant.hologram,
  ]) {
    final Tokens t = Tokens.of(variant);
    final Shapes shapes = Shapes.of(variant, t);

    testWidgets('a ${variant.name} dropdown field takes the button corner', (
      WidgetTester tester,
    ) async {
      await _pump(
        tester,
        variant,
        MenuField(open: false, onTap: () {}, child: const Text('Added')),
      );

      final Material box = _material(tester);
      final InkWell ink = tester.widget<InkWell>(
        find.descendant(
          of: find.byType(MenuField),
          matching: find.byType(InkWell),
        ),
      );

      expect(box.color, t.surface);
      expect(
        box.shape,
        shapes.border(
          Corners.button,
          side: BorderSide(color: shapes.line(t.border)),
        ),
      );
      expect(ink.customBorder, shapes.border(Corners.button));
    });

    testWidgets('hovering a ${variant.name} dropdown field shows over its '
        'fill', (WidgetTester tester) async {
      await _pump(tester, variant, _field(enabled: true));
      expect(await _paddingPixel(tester), isPixel(t.surface));

      await _hover(tester);

      expect(await _paddingPixel(tester), isNot(isPixel(t.surface)));
    });

    testWidgets('keyboard focus on a ${variant.name} dropdown field shows '
        'over its fill', (WidgetTester tester) async {
      await _pump(tester, variant, _field(enabled: true));
      expect(await _paddingPixel(tester), isPixel(t.surface));

      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pumpAndSettle();

      expect(await _paddingPixel(tester), isNot(isPixel(t.surface)));
    });

    testWidgets('pressing a ${variant.name} dropdown field shows over its '
        'fill', (WidgetTester tester) async {
      await _pump(tester, variant, _field(enabled: true));
      expect(await _paddingPixel(tester), isPixel(t.surface));

      final TestGesture press = await tester.startGesture(
        tester.getCenter(find.byType(MenuField)),
      );
      await tester.pump(kPressTimeout);
      await tester.pump(const Duration(milliseconds: 200));

      expect(await _paddingPixel(tester), isNot(isPixel(t.surface)));
      await press.up();
      await tester.pumpAndSettle();
    });

    testWidgets('a disabled ${variant.name} dropdown field shows nothing on '
        'hover and Tab passes it by', (WidgetTester tester) async {
      await _pump(tester, variant, _field(enabled: false));
      expect(_material(tester).color, t.surfaceAlt);

      await _hover(tester);

      expect(await _paddingPixel(tester), isPixel(t.surfaceAlt));

      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pumpAndSettle();

      expect(
        FocusManager.instance.primaryFocus?.context
            ?.findAncestorWidgetOfExactType<MenuField>(),
        isNull,
      );
      expect(await _paddingPixel(tester), isPixel(t.surfaceAlt));
    });

    testWidgets('a ${variant.name} toolbar placeholder takes the button '
        'corner', (WidgetTester tester) async {
      await _pump(
        tester,
        variant,
        const SkeletonPage(toolbar: true, child: SizedBox.shrink()),
      );

      final Iterable<Decoration> chips = tester
          .widgetList<DecoratedBox>(
            find.descendant(
              of: find.byType(SkeletonPage),
              matching: find.byType(DecoratedBox),
            ),
          )
          .map((DecoratedBox box) => box.decoration)
          .where(
            (Decoration d) =>
                d ==
                shapes.box(
                  Corners.button,
                  side: BorderSide(color: shapes.line(t.border)),
                ),
          );

      expect(chips, hasLength(3));
      await tester.pump(kShimmerDuration);
    });
  }
}
