import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/password_dialog.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

int _calls = 0;

ApiClient _api() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request _) async {
    _calls++;
    return http.Response('{}', 200);
  }),
);

Future<void> _pump(WidgetTester tester) async {
  _calls = 0;
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: ToastHost(
          child: Scaffold(
            body: PasswordDialog(api: _api(), userId: 'u1'),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> _submit(WidgetTester tester) async {
  await tester.tap(find.widgetWithText(FilledButton, Strings.changePassword));
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('an empty new password is rejected', (WidgetTester tester) async {
    await _pump(tester);
    await _submit(tester);

    expect(find.text(Strings.requiredNewPassword), findsOneWidget);
    expect(_calls, 0);
  });

  testWidgets('a mismatched confirmation is rejected', (
    WidgetTester tester,
  ) async {
    await _pump(tester);
    await tester.enterText(fieldNamed(Strings.fieldNewPassword), 'hunter2');
    await tester.enterText(fieldNamed(Strings.fieldConfirmPassword), 'hunter3');
    await _submit(tester);

    expect(find.text(Strings.passwordsDoNotMatch), findsOneWidget);
    expect(_calls, 0);
  });

  testWidgets('an empty current password is rejected', (
    WidgetTester tester,
  ) async {
    await _pump(tester);
    await tester.enterText(fieldNamed(Strings.fieldNewPassword), 'hunter2');
    await tester.enterText(fieldNamed(Strings.fieldConfirmPassword), 'hunter2');
    await _submit(tester);

    expect(find.text(Strings.requiredCurrentPassword), findsOneWidget);
    expect(
      _calls,
      0,
      reason:
          'a blank current password used to be sent as null and rejected '
          'by the server, which cleared the session',
    );
  });

  testWidgets('each message sits under the field it belongs to', (
    WidgetTester tester,
  ) async {
    await _pump(tester);
    await tester.enterText(fieldNamed(Strings.fieldNewPassword), 'hunter2');
    await tester.enterText(fieldNamed(Strings.fieldConfirmPassword), 'hunter3');
    await _submit(tester);

    final double current = tester
        .getTopLeft(find.text(Strings.requiredCurrentPassword))
        .dy;
    final double mismatch = tester
        .getTopLeft(find.text(Strings.passwordsDoNotMatch))
        .dy;
    final double confirmField = tester
        .getTopLeft(fieldNamed(Strings.fieldConfirmPassword))
        .dy;

    expect(current, lessThan(confirmField));
    expect(mismatch, greaterThan(confirmField));
  });

  testWidgets('a matching confirmation submits the change', (
    WidgetTester tester,
  ) async {
    await _pump(tester);
    await tester.enterText(fieldNamed(Strings.fieldCurrentPassword), 'old');
    await tester.enterText(fieldNamed(Strings.fieldNewPassword), 'hunter2');
    await tester.enterText(fieldNamed(Strings.fieldConfirmPassword), 'hunter2');
    await _submit(tester);

    expect(find.text(Strings.passwordsDoNotMatch), findsNothing);
    expect(_calls, greaterThan(0));

    await tester.pump(const Duration(seconds: 4));
  });

  testWidgets('a changed password is announced as new, not as the old one', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.byType(AutofillGroup), findsOneWidget);
    TextField field(String label) =>
        tester.widget<TextField>(fieldNamed(label));
    expect(field(Strings.fieldCurrentPassword).autofillHints, <String>[
      AutofillHints.password,
    ]);
    expect(field(Strings.fieldNewPassword).autofillHints, <String>[
      AutofillHints.newPassword,
    ], reason: 'saving this as the current password would store the stale one');
    expect(field(Strings.fieldConfirmPassword).autofillHints, <String>[
      AutofillHints.newPassword,
    ]);
  });
}
