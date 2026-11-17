import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/progress_bar.dart';
import 'package:shadowmask/components/progress_meter.dart';
import 'package:shadowmask/components/segmented_bar.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

Future<void> _pump(
  WidgetTester tester,
  AppThemeVariant variant,
  Widget child, {
  bool highContrast = false,
}) => tester.pumpWidget(
  MaterialApp(
    theme: buildTheme(variant, highContrast: highContrast),
    home: Scaffold(
      body: Center(child: SizedBox(width: 200, child: child)),
    ),
  ),
);

void _expectSegments(WidgetTester tester, Size size) {
  const Tokens t = Tokens.hologram;
  final SegmentedBar bar = tester.widget<SegmentedBar>(
    find.byType(SegmentedBar),
  );
  expect(bar.fraction, 0.4);
  expect(bar.track, t.accent.withValues(alpha: 0.26));
  expect(bar.fill, t.accent);
  expect(tester.getSize(find.byType(SegmentedBar)), size);
}

void main() {
  testWidgets('hologram shows watch progress as a segmented bar', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AppThemeVariant.hologram,
      const ProgressBar(percent: 40),
    );

    _expectSegments(tester, const Size(200, kSegmentedBarHeight));
  });

  testWidgets('hologram shows job progress as a segmented bar', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AppThemeVariant.hologram,
      const ProgressMeter(percent: 40, width: 90),
    );

    _expectSegments(tester, const Size(90, kSegmentedBarHeight));
    expect(find.text('40%'), findsOneWidget);
  });

  for (final Widget bar in const <Widget>[
    ProgressBar(percent: 40),
    ProgressMeter(percent: 40, width: 90),
  ]) {
    testWidgets('high contrast hologram keeps the ${bar.runtimeType} track '
        'apart from its fill', (WidgetTester tester) async {
      await _pump(tester, AppThemeVariant.hologram, bar, highContrast: true);
      const Tokens t = Tokens.hologram;
      final SegmentedBar segments = tester.widget<SegmentedBar>(
        find.byType(SegmentedBar),
      );

      expect(segments.track, t.accent.withValues(alpha: 0.26));
      expect(segments.fill, t.accent);
    });
  }

  for (final AppThemeVariant variant in AppThemeVariant.values.where(
    (AppThemeVariant v) => v != AppThemeVariant.hologram,
  )) {
    testWidgets('${variant.name} keeps the plain progress bars', (
      WidgetTester tester,
    ) async {
      await _pump(tester, variant, const ProgressBar(percent: 40));
      expect(find.byType(SegmentedBar), findsNothing);
      expect(
        tester.getSize(find.byType(ProgressBar)),
        const Size(200, kProgressBarHeight),
      );

      await _pump(tester, variant, const ProgressMeter(percent: 40));
      expect(find.byType(SegmentedBar), findsNothing);
      expect(
        tester
            .widget<FractionallySizedBox>(find.byType(FractionallySizedBox))
            .widthFactor,
        0.4,
      );
    });
  }
}
