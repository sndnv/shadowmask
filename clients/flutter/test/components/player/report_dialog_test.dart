import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/components/player/playback_log_dialog.dart';
import 'package:shadowmask/components/player/report_dialog.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

const Map<String, Object?> _payload = <String, Object?>{
  'schema': 1,
  'log': <String>['09:00:01 waiting'],
};

Future<void> _open(WidgetTester tester, ReportSender send) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      builder: (BuildContext context, Widget? child) =>
          ToastHost(child: child!),
      home: Builder(
        builder: (BuildContext context) => TextButton(
          onPressed: () =>
              showReportDialog(context, payload: _payload, send: send),
          child: const Text('open'),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('a blank note is sent as none', (WidgetTester tester) async {
    final List<String?> notes = <String?>[];
    await _open(tester, (String? note) async => notes.add(note));

    await tester.enterText(find.byType(TextField), '   ');
    await tester.tap(find.text(Strings.reportSend));
    await tester.pumpAndSettle();

    expect(notes, <String?>[null]);
    expect(find.text(Strings.reportTitle), findsNothing);
  });

  testWidgets('a send that fails keeps the dialog and the note', (
    WidgetTester tester,
  ) async {
    int tries = 0;
    await _open(tester, (String? _) async {
      tries += 1;
      throw ApiException('nope', status: 500);
    });

    await tester.enterText(find.byType(TextField), 'stutters');
    await tester.tap(find.text(Strings.reportSend));
    await tester.pumpAndSettle();

    expect(tries, 1);
    expect(find.text(Strings.reportTitle), findsOneWidget);
    expect(find.text('stutters'), findsOneWidget);
    expect(find.textContaining(Strings.errorSendReport), findsOneWidget);
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNotNull,
      reason: 'it can be sent again',
    );
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('cancel sends nothing', (WidgetTester tester) async {
    int tries = 0;
    await _open(tester, (String? _) async => tries += 1);

    await tester.tap(find.text(Strings.cancel));
    await tester.pumpAndSettle();

    expect(tries, 0);
    expect(find.text(Strings.reportTitle), findsNothing);
  });

  testWidgets('the playback log offers a report only where it can be sent', (
    WidgetTester tester,
  ) async {
    final List<bool> answers = <bool>[];
    Future<void> open(bool canReport) async {
      await tester.pumpWidget(
        MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: Builder(
            builder: (BuildContext context) => TextButton(
              onPressed: () async => answers.add(
                await showPlaybackLogDialog(context, const <String>[
                  'a line',
                ], canReport: canReport),
              ),
              child: const Text('open'),
            ),
          ),
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
    }

    await open(false);
    expect(find.text(Strings.sendReport), findsNothing);
    await tester.tap(find.byIcon(Icons.close));
    await tester.pumpAndSettle();

    await open(true);
    await tester.tap(find.text(Strings.sendReport));
    await tester.pumpAndSettle();

    expect(answers, <bool>[false, true]);
  });
}
