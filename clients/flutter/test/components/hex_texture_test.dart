import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:material_ui/material_ui.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/hex_texture.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';

void main() {
  Future<void> pumpTexture(
    WidgetTester tester, {
    AppThemeVariant variant = AppThemeVariant.dark,
  }) async {
    await tester.pumpWidget(
      ThemeScope(
        variant: variant,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(variant),
          home: const Scaffold(body: HexTexture()),
        ),
      ),
    );
    await tester.pump();
  }

  Finder inTexture(Type type) =>
      find.descendant(of: find.byType(HexTexture), matching: find.byType(type));

  Future<void> expectHexagonInk(
    WidgetTester tester,
    Color ink,
    double alpha,
  ) async {
    final RenderRepaintBoundary boundary = tester.renderObject(
      inTexture(RepaintBoundary),
    );
    final ByteData? pixels = await tester.runAsync<ByteData?>(() async {
      final ui.Image image = await boundary.toImage();
      return image.toByteData();
    });
    final int at = (12 * boundary.size.width.toInt() + 7) * 4;
    final List<double> expected = <double>[
      ink.r * alpha * 255,
      ink.g * alpha * 255,
      ink.b * alpha * 255,
      alpha * 255,
    ];
    for (int channel = 0; channel < 4; channel++) {
      expect(
        pixels!.getUint8(at + channel),
        closeTo(expected[channel], 2),
        reason: 'channel $channel',
      );
    }
  }

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

  testWidgets('dark paints its hexagons in the text colour at 6%', (
    WidgetTester tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetDevicePixelRatio);
    await pumpTexture(tester);

    await expectHexagonInk(tester, Tokens.dark.text, 0.06);
  });

  testWidgets('hologram paints its hexagons in the accent at 11%', (
    WidgetTester tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetDevicePixelRatio);
    await pumpTexture(tester, variant: AppThemeVariant.hologram);

    await expectHexagonInk(tester, Tokens.hologram.accent, 0.11);
  });
}
