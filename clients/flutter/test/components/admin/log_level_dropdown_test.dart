import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/log_level_dropdown.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

void main() {
  testWidgets('shows the level and reports the one picked', (
    WidgetTester tester,
  ) async {
    final List<String> picked = <String>[];
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: LogLevelDropdown(
            value: kDefaultLogLevel,
            onChanged: picked.add,
          ),
        ),
      ),
    );

    expect(find.text('Info'), findsOneWidget);
    await tester.tap(find.text('Info'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Warn').last);
    await tester.pumpAndSettle();

    expect(picked, <String>['WARN']);
  });
}
