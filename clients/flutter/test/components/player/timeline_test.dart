import 'package:flutter/gestures.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/player/timeline.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

const Color _halo = Color(0x4D000000);

Finder get _track => find.descendant(
  of: find.byType(Timeline),
  matching: find.byType(CustomPaint),
);

PaintPatternPredicate _caps(StrokeCap cap) =>
    (Symbol method, List<dynamic> args) =>
        method != #drawLine || (args[2] as Paint).strokeCap == cap;

Widget _host({
  required void Function(double)? onSeek,
  bool withThumb = true,
  AppThemeVariant variant = AppThemeVariant.dark,
  bool highContrast = false,
}) => MaterialApp(
  theme: buildTheme(variant, highContrast: highContrast),
  home: Scaffold(
    body: Center(
      child: SizedBox(
        width: 400,
        child: Timeline(
          fraction: 0.5,
          bufferedFraction: 0.6,
          durationMs: 100000,
          onSeek: onSeek,
          thumbBuilder: withThumb
              ? (int ms) =>
                    const SizedBox(key: Key('thumb'), width: 160, height: 90)
              : null,
        ),
      ),
    ),
  ),
);

void main() {
  testWidgets('a cancelled touch does not strand the preview', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(onSeek: (_) {}));

    final TestGesture touch = await tester.startGesture(
      tester.getCenter(find.byType(Timeline)),
      kind: PointerDeviceKind.touch,
    );
    await tester.pump();
    await touch.moveBy(const Offset(24, 0));
    await tester.pump();
    await touch.moveBy(const Offset(12, 0));
    await tester.pump();
    expect(find.byKey(const Key('thumb')), findsOneWidget);

    // A touch has no pointer exit, so only a cancel can clear this.
    await touch.cancel();
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('thumb')), findsNothing);
  });

  testWidgets('a touch preview fades on its own when nothing follows', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(onSeek: (_) {}));

    final TestGesture touch = await tester.startGesture(
      tester.getCenter(find.byType(Timeline)),
      kind: PointerDeviceKind.touch,
    );
    await tester.pump();
    await touch.moveBy(const Offset(24, 0));
    await tester.pump();
    await touch.moveBy(const Offset(12, 0));
    await tester.pump();
    expect(find.byKey(const Key('thumb')), findsOneWidget);

    // A lifted touch may fire neither an end nor a cancel, so the preview has
    // to time itself out or it stays on screen for good.
    await tester.pump(const Duration(milliseconds: 1200));

    expect(find.byKey(const Key('thumb')), findsNothing);
    await touch.cancel();
    await tester.pumpAndSettle();
  });

  testWidgets('a hovering pointer keeps the preview up', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(onSeek: (_) {}));

    final TestGesture pointer = await tester.createGesture(
      kind: PointerDeviceKind.mouse,
    );
    await pointer.addPointer(location: Offset.zero);
    addTearDown(pointer.removePointer);
    await tester.pump();

    await pointer.moveTo(tester.getCenter(find.byType(Timeline)));
    await tester.pump();
    expect(find.byKey(const Key('thumb')), findsOneWidget);

    await tester.pump(const Duration(milliseconds: 1200));

    expect(
      find.byKey(const Key('thumb')),
      findsOneWidget,
      reason: 'a still mouse over the bar is still asking to see the preview',
    );
  });

  // The preview follows the pointer while playback stays where it is, so the
  // popup has to be placed from the hover position and not from the fraction.
  testWidgets('the preview popup tracks the pointer, not playback', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(onSeek: (_) {}));

    final TestGesture pointer = await tester.createGesture(
      kind: PointerDeviceKind.mouse,
    );
    await pointer.addPointer(location: Offset.zero);
    addTearDown(pointer.removePointer);
    await tester.pump();

    final Offset centre = tester.getCenter(find.byType(Timeline));
    await pointer.moveTo(centre);
    await tester.pump();
    final double middle = tester.getTopLeft(find.byKey(const Key('thumb'))).dx;

    await pointer.moveTo(centre + const Offset(100, 0));
    await tester.pump();
    final double later = tester.getTopLeft(find.byKey(const Key('thumb'))).dx;

    expect(later - middle, closeTo(100, 0.5));
  });

  testWidgets('a completed tap seeks and clears the preview', (
    WidgetTester tester,
  ) async {
    double? sought;
    await tester.pumpWidget(_host(onSeek: (double f) => sought = f));

    await tester.tapAt(tester.getCenter(find.byType(Timeline)));
    await tester.pumpAndSettle();

    expect(sought, closeTo(0.5, 0.02));
    expect(find.byKey(const Key('thumb')), findsNothing);
  });

  testWidgets('the timeline reports itself as a slider with a position', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await tester.pumpWidget(_host(onSeek: (_) {}));

    // A custom painter says nothing on its own, so this is the whole contract:
    // a role, where playback is, and where a step would land.
    expect(
      tester.getSemantics(find.bySemanticsLabel(Strings.timelineLabel)),
      matchesSemantics(
        isSlider: true,
        label: Strings.timelineLabel,
        value: '0:50',
        increasedValue: '1:00',
        decreasedValue: '0:40',
        hasIncreaseAction: true,
        hasDecreaseAction: true,
        hasTapAction: true,
        hasScrollLeftAction: true,
        hasScrollRightAction: true,
      ),
    );

    semantics.dispose();
  });

  testWidgets('a timeline that cannot seek is greyed and takes no input', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await tester.pumpWidget(_host(onSeek: null));

    await tester.tapAt(tester.getCenter(find.byType(Timeline)));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('thumb')), findsNothing);
    expect(
      tester
          .widget<Opacity>(
            find.descendant(
              of: find.byType(Timeline),
              matching: find.byType(Opacity),
            ),
          )
          .opacity,
      0.5,
    );
    expect(
      tester
          .getSemantics(find.bySemanticsLabel(Strings.timelineLabel))
          .getSemanticsData()
          .hasAction(SemanticsAction.increase),
      isFalse,
    );
    semantics.dispose();
  });

  testWidgets('hologram draws a hexagon playhead over a tick every 5%', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(onSeek: (_) {}, variant: AppThemeVariant.hologram),
    );
    final Color accent = Tokens.hologram.accent;
    final PaintPattern pattern = paints;
    for (final double x in <double>[
      for (int i = 0; i < 20; i++) i * 20.0,
      399,
    ]) {
      pattern.rect(
        rect: Rect.fromLTWH(x, 19, 1, 4),
        color: accent.withValues(alpha: 0.26),
      );
    }
    pattern
      ..path(
        color: _halo,
        includes: const <Offset>[Offset(200, 14), Offset(211, 14)],
        excludes: const <Offset>[Offset(213, 14), Offset(189, 4)],
      )
      ..path(
        color: accent,
        includes: const <Offset>[Offset(200, 14), Offset(207, 14)],
        excludes: const <Offset>[Offset(209, 14), Offset(193, 8)],
      );

    expect(_track, pattern);
    expect(_track, paintsExactlyCountTimes(#drawRect, 21));
    expect(_track, paintsExactlyCountTimes(#drawCircle, 0));
    expect(_track, paints..everything(_caps(StrokeCap.butt)));
  });

  testWidgets('hologram draws the preview playhead as a hexagon too', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(onSeek: (_) {}, variant: AppThemeVariant.hologram),
    );
    final TestGesture pointer = await tester.createGesture(
      kind: PointerDeviceKind.mouse,
    );
    await pointer.addPointer(location: Offset.zero);
    addTearDown(pointer.removePointer);
    await pointer.moveTo(
      tester.getCenter(find.byType(Timeline)) + const Offset(100, 0),
    );
    await tester.pump();
    final Color accent = Tokens.hologram.accent;

    expect(
      _track,
      paints
        ..path(color: _halo, includes: const <Offset>[Offset(311, 14)])
        ..path(
          color: accent.withValues(alpha: 0.45),
          includes: const <Offset>[Offset(300, 14), Offset(307, 14)],
          excludes: const <Offset>[Offset(309, 14)],
        )
        ..path(color: _halo, includes: const <Offset>[Offset(211, 14)])
        ..path(color: accent, includes: const <Offset>[Offset(207, 14)]),
    );
    expect(_track, paintsExactlyCountTimes(#drawCircle, 0));
  });

  for (final AppThemeVariant variant in AppThemeVariant.values.where(
    (AppThemeVariant v) => v != AppThemeVariant.hologram,
  )) {
    testWidgets('${variant.name} keeps the round playhead and no ticks', (
      WidgetTester tester,
    ) async {
      await tester.pumpWidget(_host(onSeek: (_) {}, variant: variant));

      expect(
        _track,
        paints
          ..circle(x: 200, y: 14, radius: 9, color: _halo)
          ..circle(x: 200, y: 14, radius: 6, color: Tokens.of(variant).accent),
      );
      expect(_track, paintsExactlyCountTimes(#drawRect, 0));
      expect(_track, paintsExactlyCountTimes(#drawPath, 0));
      expect(_track, paints..everything(_caps(StrokeCap.round)));
    });
  }

  testWidgets('a theme change repaints a paused timeline', (
    WidgetTester tester,
  ) async {
    Future<CustomPainter> painterOf(
      AppThemeVariant variant, {
      bool highContrast = false,
    }) async {
      await tester.pumpWidget(
        _host(onSeek: (_) {}, variant: variant, highContrast: highContrast),
      );
      await tester.pumpAndSettle();
      return tester.renderObject<RenderCustomPaint>(_track).painter!;
    }

    final CustomPainter dark = await painterOf(AppThemeVariant.dark);
    final CustomPainter light = await painterOf(AppThemeVariant.light);
    final CustomPainter hologram = await painterOf(AppThemeVariant.hologram);
    final CustomPainter hardened = await painterOf(
      AppThemeVariant.hologram,
      highContrast: true,
    );
    final CustomPainter again = await painterOf(AppThemeVariant.hologram);

    expect(light.shouldRepaint(dark), isTrue);
    expect(hologram.shouldRepaint(light), isTrue);
    expect(hardened.shouldRepaint(hologram), isTrue);
    expect(again, isNot(same(hologram)));
    expect(again.shouldRepaint(hologram), isFalse);
  });
}
