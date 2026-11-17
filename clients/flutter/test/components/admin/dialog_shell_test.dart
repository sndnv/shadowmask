import 'dart:math' as math;

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/title_bar.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';

import '../../support/raster.dart';

const Key _frame = ValueKey<String>('frame');

Future<void> _pump(WidgetTester tester, AppThemeVariant variant) async {
  await tester.pumpWidget(
    ThemeScope(
      variant: variant,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(variant),
        home: const Scaffold(
          body: RepaintBoundary(
            key: _frame,
            child: DialogShell(
              title: 'Delete subtitle',
              child: SizedBox(height: 120, child: Text('Body')),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Finder _panel() => find.byWidgetPredicate(
  (Widget w) => w is Container && w.clipBehavior == Clip.antiAlias,
);

void main() {
  testWidgets('a hologram title bar runs to the outline and the edge shows '
      'below it', (WidgetTester tester) async {
    await _pump(tester, AppThemeVariant.hologram);
    const Tokens t = Tokens.hologram;
    final Offset at =
        tester.getTopLeft(_panel()) - tester.getTopLeft(find.byKey(_frame));
    final Size size = tester.getSize(_panel());
    final double step = 0.5 / math.sqrt2;

    final List<Color> pixels =
        await pixelsAt(tester, find.byKey(_frame), <Offset>[
          at + Offset(7 + step, 7 + step),
          at + Offset(size.width / 2, 0.5),
          at + const Offset(0.5, 20),
          at + Offset(0.5, size.height - 10),
        ]);

    expect(pixels, <Matcher>[
      isPixel(t.accent),
      isPixel(t.accent),
      isPixel(t.accent),
      isPixel(Color.alphaBlend(t.accent.withValues(alpha: 0.62), t.surface)),
    ]);
  });

  for (final AppThemeVariant variant in <AppThemeVariant>[
    AppThemeVariant.dark,
    AppThemeVariant.hologram,
  ]) {
    testWidgets('a ${variant.name} dialog heads with the shared title bar', (
      WidgetTester tester,
    ) async {
      await _pump(tester, variant);

      expect(
        find.descendant(
          of: find.byType(DialogShell),
          matching: find.byType(TitleBar),
        ),
        findsOneWidget,
      );
      expect(
        tester.getSize(find.byType(TitleBar)).width,
        tester.getSize(_panel()).width -
            (variant == AppThemeVariant.hologram ? 0 : 2),
      );
    });
  }

  testWidgets('a rounded dialog keeps its border under the content', (
    WidgetTester tester,
  ) async {
    await _pump(tester, AppThemeVariant.dark);

    final Container panel = tester.widget<Container>(_panel());

    expect(panel.foregroundDecoration, isNull);
    expect((panel.decoration! as BoxDecoration).border, isNotNull);
  });
}
