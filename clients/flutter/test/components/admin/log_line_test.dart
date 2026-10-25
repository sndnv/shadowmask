import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/log_line.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

Future<Color?> _colorOf(WidgetTester tester, String entry) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(body: LogLine(entry)),
    ),
  );
  return tester.widget<Text>(find.byType(Text)).style?.color;
}

void main() {
  testWidgets('each level of a server log entry has its colour', (
    WidgetTester tester,
  ) async {
    const String at = '2026-10-24T09:00:00.000001Z';
    expect(
      await _colorOf(tester, '$at DEBUG api: heartbeat'),
      Tokens.dark.muted,
    );
    expect(await _colorOf(tester, '$at  INFO api: started'), Tokens.dark.text);
    expect(await _colorOf(tester, '$at  WARN media: slow'), Tokens.dark.warn);
    expect(await _colorOf(tester, '$at ERROR jobs: boom'), Tokens.dark.danger);
  });

  testWidgets('a job log line and a multi-line entry take their first '
      'line\'s level', (WidgetTester tester) async {
    expect(
      await _colorOf(tester, '1970-01-01T00:00:00Z WARN slow'),
      Tokens.dark.warn,
    );
    expect(
      await _colorOf(tester, '2026-10-24T09:00:00Z ERROR x: boom\n  DEBUG no'),
      Tokens.dark.danger,
    );
  });

  testWidgets('a line with no level is plain text', (
    WidgetTester tester,
  ) async {
    expect(await _colorOf(tester, 'orphan'), Tokens.dark.text);
    expect(await _colorOf(tester, 'frame=12 speed=0.9x'), Tokens.dark.text);
  });
}
