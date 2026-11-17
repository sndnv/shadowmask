import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/job_status_chip.dart';
import 'package:shadowmask/components/status_node.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/tokens.dart';

Future<void> _pump(
  WidgetTester tester,
  AppThemeVariant variant,
  JobStatus status,
) => tester.pumpWidget(
  MaterialApp(
    theme: buildTheme(variant),
    home: Scaffold(body: Center(child: JobStatusChip(status))),
  ),
);

(Color, Color, String) _look(Tokens t, JobStatus status) => switch (status) {
  JobStatus.queued => (t.warn, t.warnBg, Strings.statusQueued),
  JobStatus.running => (t.accent, t.okBg, Strings.statusRunning),
  JobStatus.succeeded => (t.ok, t.okBg, Strings.statusSucceeded),
  JobStatus.failed => (t.danger, t.dangerBg, Strings.statusFailed),
  JobStatus.cancelled => (t.muted, t.surfaceAlt, Strings.statusCancelled),
};

Container _chip(WidgetTester tester) => tester.widget<Container>(
  find
      .descendant(
        of: find.byType(JobStatusChip),
        matching: find.byType(Container),
      )
      .first,
);

void main() {
  for (final JobStatus status in JobStatus.values) {
    testWidgets('hologram shows ${status.name} as a node and a label', (
      WidgetTester tester,
    ) async {
      await _pump(tester, AppThemeVariant.hologram, status);
      const Tokens t = Tokens.hologram;
      final (Color mark, Color _, String label) = _look(t, status);

      expect(tester.widget<StatusNode>(find.byType(StatusNode)).color, mark);
      expect(tester.widget<Text>(find.text(label)).style?.color, t.text);
      expect(_chip(tester).decoration, isNull);
    });
  }

  testWidgets('a hologram status starts where its column starts', (
    WidgetTester tester,
  ) async {
    await _pump(tester, AppThemeVariant.hologram, JobStatus.running);

    expect(
      tester.getTopLeft(find.byType(StatusNode)).dx,
      tester.getTopLeft(find.byType(JobStatusChip)).dx,
    );
  });

  testWidgets('a status node is the mock\'s small hexagon', (
    WidgetTester tester,
  ) async {
    await _pump(tester, AppThemeVariant.hologram, JobStatus.failed);

    final Finder node = find.byType(StatusNode);
    expect(tester.getSize(node), Marks.node);
    expect(
      tester
          .widget<Container>(
            find.descendant(of: node, matching: find.byType(Container)),
          )
          .decoration,
      ShapeDecoration(
        color: Tokens.hologram.danger,
        shape: Shapes.hexagon(Marks.node),
      ),
    );
  });

  for (final AppThemeVariant variant in AppThemeVariant.values.where(
    (AppThemeVariant v) => v != AppThemeVariant.hologram,
  )) {
    testWidgets('${variant.name} keeps the tinted status chip', (
      WidgetTester tester,
    ) async {
      final Tokens t = Tokens.of(variant);
      for (final JobStatus status in JobStatus.values) {
        await _pump(tester, variant, status);
        final (Color ink, Color ground, String label) = _look(t, status);

        expect(find.byType(StatusNode), findsNothing);
        expect(tester.widget<Text>(find.text(label)).style?.color, ink);
        expect((_chip(tester).decoration as BoxDecoration?)?.color, ground);
      }
    });
  }
}
