import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadowmask/components/edged_box.dart';
import 'package:shadowmask/components/player/trickplay_thumb.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/tokens.dart';

void main() {
  for (final AppThemeVariant variant in <AppThemeVariant>[
    AppThemeVariant.dark,
    AppThemeVariant.hologram,
  ]) {
    testWidgets('the ${variant.name} thumb is an edged inset box', (
      WidgetTester tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          theme: buildTheme(variant),
          home: const Scaffold(
            body: Center(child: TrickplayThumb(tile: null, positionMs: 61000)),
          ),
        ),
      );
      final Tokens t = Tokens.of(variant);

      final EdgedBox box = tester.widget<EdgedBox>(find.byType(EdgedBox));

      expect(box.corner, Corners.inset);
      expect(
        box.edge,
        BorderSide(color: Shapes.of(variant, t).lineSoft(t.border)),
      );
      expect(tester.getSize(find.byType(EdgedBox)), const Size(168, 94.5));
    });
  }
}
