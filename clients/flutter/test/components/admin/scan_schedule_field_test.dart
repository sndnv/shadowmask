import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/scan_schedule_field.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Future<String?> _pump(WidgetTester tester, String initial) async {
  String? written;
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: ScanScheduleField(
          value: initial,
          onChanged: (String v) => written = v,
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return written;
}

void main() {
  testWidgets('a preset writes its cron expression', (
    WidgetTester tester,
  ) async {
    String? written;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ScanScheduleField(
            value: '',
            onChanged: (String v) => written = v,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.scheduleOff), findsOneWidget);

    await tester.tap(find.text(Strings.scheduleOff));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.scheduleDaily).last);
    await tester.pumpAndSettle();

    expect(written, '0 0 3 * * * *');
    expect(find.byType(TextField), findsNothing);
    expect(find.text(Strings.scheduleDaily), findsOneWidget);
    expect(find.text(Strings.scheduleOff), findsNothing);
  });

  testWidgets('a new value from the parent replaces the selection', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ScanScheduleField(value: '', onChanged: (String _) {}),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ScanScheduleField(
            value: '0 0 3 * * Sun *',
            onChanged: (String _) {},
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.scheduleWeekly), findsOneWidget);
  });

  testWidgets('Custom reveals the expression box, prefilled with daily', (
    WidgetTester tester,
  ) async {
    String? written;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ScanScheduleField(
            value: '',
            onChanged: (String v) => written = v,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.scheduleOff));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.scheduleCustom).last);
    await tester.pumpAndSettle();

    expect(find.byType(TextField), findsOneWidget);
    expect(
      tester.widget<TextField>(find.byType(TextField)).controller?.text,
      kScheduleDefaultCustom,
    );
    expect(written, kScheduleDefaultCustom);

    await tester.enterText(find.byType(TextField), '30 0 4 * * * *');
    expect(written, '30 0 4 * * * *');
  });

  testWidgets('an expression that matches a preset opens on that preset', (
    WidgetTester tester,
  ) async {
    await _pump(tester, '0 0 */6 * * * *');

    expect(find.text(Strings.scheduleSixHourly), findsOneWidget);
    expect(find.byType(TextField), findsNothing);
  });

  testWidgets('an unrecognised expression opens as Custom', (
    WidgetTester tester,
  ) async {
    await _pump(tester, '15 30 2 * * Mon *');

    expect(find.text(Strings.scheduleCustom), findsOneWidget);
    expect(
      tester.widget<TextField>(find.byType(TextField)).controller?.text,
      '15 30 2 * * Mon *',
    );
  });

  testWidgets('the help dialog explains the seven fields', (
    WidgetTester tester,
  ) async {
    await _pump(tester, '');

    await tester.tap(find.byIcon(Icons.help_outline));
    await tester.pumpAndSettle();

    expect(find.textContaining('seven fields'), findsOneWidget);
  });
}
