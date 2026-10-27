import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/hex_texture.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

void main() {
  Future<void> pumpTexture(WidgetTester tester) async {
    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: const Scaffold(body: HexTexture()),
        ),
      ),
    );
    await tester.pump();
  }

  Finder inTexture(Type type) =>
      find.descendant(of: find.byType(HexTexture), matching: find.byType(type));

  testWidgets('paints on its own layer and ignores pointer events', (
    WidgetTester tester,
  ) async {
    await pumpTexture(tester);

    expect(find.byType(HexTexture), findsOneWidget);
    expect(inTexture(RepaintBoundary), findsOneWidget);
    expect(inTexture(IgnorePointer), findsOneWidget);
    expect(inTexture(CustomPaint), findsOneWidget);
  });

  testWidgets('fills its area with one shaded rectangle and no path', (
    WidgetTester tester,
  ) async {
    await pumpTexture(tester);

    final RenderObject painter = tester.renderObject(inTexture(CustomPaint));
    expect(
      painter,
      paints..rect(rect: Offset.zero & tester.getSize(find.byType(HexTexture))),
    );
    expect(painter, paintsExactlyCountTimes(#drawPath, 0));
  });

  testWidgets('repeats the hexagons across tiles without a seam', (
    WidgetTester tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetDevicePixelRatio);
    await pumpTexture(tester);

    final RenderRepaintBoundary boundary = tester.renderObject(
      inTexture(RepaintBoundary),
    );
    final ByteData? pixels = await tester.runAsync<ByteData?>(() async {
      final ui.Image image = await boundary.toImage();
      return image.toByteData();
    });
    final int width = boundary.size.width.toInt();
    int alpha(int x, int y) => pixels!.getUint8((y * width + x) * 4 + 3);

    expect(alpha(0, 0), greaterThan(0));
    expect(alpha(7, 12), greaterThan(0));
    expect(alpha(14, 24), greaterThan(0));
    expect(alpha(21, 36), greaterThan(0));
    expect(alpha(7, 0), 0);
    expect(alpha(14, 12), 0);
  });
}
