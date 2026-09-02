import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/player/timeline.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Widget _host({required void Function(double) onSeek, bool withThumb = true}) =>
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
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
                  ? (int ms) => const SizedBox(
                      key: Key('thumb'),
                      width: 160,
                      height: 90,
                    )
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
}
