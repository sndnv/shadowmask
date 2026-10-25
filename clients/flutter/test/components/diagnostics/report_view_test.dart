import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/diagnostics/report_view.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

void main() {
  test('lines are one per row, entries lead with their time and event', () {
    expect(reportSectionText(<String>['a', 'b']), 'a\nb');
    expect(
      reportSectionText(<Map<String, dynamic>>[
        <String, dynamic>{'at': 't1', 'event': 'opened'},
        <String, dynamic>{'at': 't2', 'event': 'sent_seek', 'position_ms': 5},
      ]),
      't1 opened\nt2 sent_seek {"position_ms":5}',
    );
    expect(
      reportSectionText(<String, dynamic>{'app': 'flutter'}),
      '{\n  "app": "flutter"\n}',
    );
    expect(reportSectionText(<String>[]), isEmpty);
  });

  testWidgets('each section has its heading, and an empty one says so', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: const Scaffold(
          body: SingleChildScrollView(
            child: ReportView(
              payload: <String, Object?>{
                'schema': 1,
                'client': <String, Object?>{'app': 'flutter'},
                'history': <Map<String, Object?>>[
                  <String, Object?>{'at': 't1', 'event': 'opened'},
                ],
                'panel': <String>[],
                'log': <String>['09:00:01 waiting'],
                'extra': 'from a newer client',
              },
            ),
          ),
        ),
      ),
    );

    for (final String heading in <String>[
      Strings.reportClient,
      Strings.reportHistory,
      Strings.reportPanel,
      Strings.playerLog,
      'extra',
    ]) {
      expect(find.text(heading), findsOneWidget, reason: heading);
    }
    expect(find.text('schema'), findsNothing);
    expect(find.text('t1 opened'), findsOneWidget);
    expect(find.text('09:00:01 waiting'), findsOneWidget);
    expect(find.text(Strings.reportNothing), findsOneWidget);
    expect(find.text('"from a newer client"'), findsOneWidget);
  });
}
