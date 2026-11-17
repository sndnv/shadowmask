import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/resume_ring.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/tokens.dart';

import '../support/shape_guard.dart';

Future<void> _pump(WidgetTester tester, {int? percent = 40}) =>
    tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.hologram),
        home: Scaffold(
          body: Center(child: ResumeRing(percent: percent)),
        ),
      ),
    );

Finder get _painted => find.descendant(
  of: find.byType(ResumeRing),
  matching: find.byWidgetPredicate(
    (Widget w) => w is CustomPaint && w.painter != null,
  ),
);

Map<int, int> _arcs(WidgetTester tester, double strokeWidth) {
  final Map<int, int> counts = <int, int>{};
  expect(
    _painted,
    paints..everything((Symbol method, List<dynamic> args) {
      if (method == #drawArc) {
        final Paint paint = args[4] as Paint;
        if (paint.strokeWidth == strokeWidth) {
          counts.update(
            paint.color.toARGB32(),
            (int n) => n + 1,
            ifAbsent: () => 1,
          );
        }
      }
      return true;
    }),
  );
  return counts;
}

void main() {
  testWidgets('the ring is the mock\'s size', (WidgetTester tester) async {
    await _pump(tester);

    expect(
      tester.getSize(find.byType(ResumeRing)),
      const Size.square(kResumeRingSize),
    );
  });

  testWidgets('forty segments, lit in the accent up to the progress', (
    WidgetTester tester,
  ) async {
    await _pump(tester);
    const Tokens t = Tokens.hologram;

    expect(_arcs(tester, 8), <int, int>{
      t.accent.toARGB32(): 16,
      t.accent.withValues(alpha: 0.26).toARGB32(): 24,
    });
  });

  testWidgets('the lit segments run on from the top', (
    WidgetTester tester,
  ) async {
    await _pump(tester);
    const Tokens t = Tokens.hologram;
    final List<int> order = <int>[];
    expect(
      _painted,
      paints..everything((Symbol method, List<dynamic> args) {
        if (method == #drawArc && (args[4] as Paint).strokeWidth == 8) {
          order.add((args[4] as Paint).color.toARGB32());
        }
        return true;
      }),
    );

    expect(order, <int>[
      for (int i = 0; i < 16; i++) t.accent.toARGB32(),
      for (int i = 16; i < 40; i++) t.accent.withValues(alpha: 0.26).toARGB32(),
    ]);
  });

  testWidgets('the first segment starts at the top and runs clockwise', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(
      _painted,
      paints..arc(
        rect: Rect.fromCircle(
          center: const Offset(kResumeRingSize / 2, kResumeRingSize / 2),
          radius: 60,
        ),
        startAngle: -math.pi / 2,
        sweepAngle: 7 / 60,
        strokeWidth: 8,
      ),
    );
  });

  testWidgets('a dashed inner ring runs in the line', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(_arcs(tester, 1), <int, int>{
      Tokens.hologram.accent.withValues(alpha: 0.62).toARGB32(): 64,
    });
  });

  testWidgets('the core reads Resume over the percent in monospace', (
    WidgetTester tester,
  ) async {
    await _pump(tester, percent: 64);

    expect(find.text(Strings.resumeAction), findsOneWidget);
    expect(find.byIcon(Icons.play_arrow), findsOneWidget);
    final Text reading = tester.widget<Text>(find.text('64%'));
    expect(reading.style?.fontFamily, kMonoFamily);
    expect(
      reading.style?.color,
      Tokens.hologram.accentContrast.withValues(alpha: 0.85),
    );
  });

  testWidgets('without progress the core reads Play over an empty ring', (
    WidgetTester tester,
  ) async {
    await _pump(tester, percent: null);

    expect(find.text(Strings.play), findsOneWidget);
    expect(find.text(Strings.resumeAction), findsNothing);
    expect(find.textContaining('%'), findsNothing);
    expect(_arcs(tester, 8), <int, int>{
      Tokens.hologram.accent.withValues(alpha: 0.26).toARGB32(): 40,
    });
  });

  testWidgets('the ring is only a picture', (WidgetTester tester) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(tester);

    expect(
      find.descendant(
        of: find.byType(ResumeRing),
        matching: find.byType(InkWell),
      ),
      findsNothing,
    );
    expect(find.bySemanticsLabel(Strings.resumeAction), findsNothing);
    expect(find.bySemanticsLabel('40%'), findsNothing);
    semantics.dispose();
  });

  testWidgets('the ring keeps its size at a large text size', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.hologram),
        home: const MediaQuery(
          data: MediaQueryData(textScaler: TextScaler.linear(2)),
          child: Scaffold(body: Center(child: ResumeRing(percent: 40))),
        ),
      ),
    );

    expect(
      tester.getSize(find.byType(ResumeRing)),
      const Size.square(kResumeRingSize),
    );
    expect(tester.takeException(), isNull);
    expect(
      tester.getSize(find.text(Strings.resumeAction)).height,
      lessThan(20),
    );
  });

  testWidgets('the ring is the one round shape hologram allows', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expectShapesOf(tester, AppThemeVariant.hologram);
  });

  testWidgets('a circle beside the ring is still caught', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.hologram),
        home: const Scaffold(
          body: Row(
            children: <Widget>[
              ResumeRing(percent: 40),
              Material(
                shape: CircleBorder(),
                child: SizedBox.square(dimension: 20),
              ),
            ],
          ),
        ),
      ),
    );

    expect(
      () => expectShapesOf(tester, AppThemeVariant.hologram),
      throwsA(isA<TestFailure>()),
    );
  });
}
