import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/log_line.dart';
import 'package:shadowmask/components/admin/log_view.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

void main() {
  testWidgets('every entry is a line of its own, in order', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: const Scaffold(
          body: LogView(entries: <String>['first', 'second\n  more']),
        ),
      ),
    );

    final List<LogLine> lines = tester
        .widgetList<LogLine>(find.byType(LogLine))
        .toList();
    expect(lines.map((LogLine line) => line.entry), <String>[
      'first',
      'second\n  more',
    ]);
    expect(find.byType(SelectionArea), findsOneWidget);
  });
}
