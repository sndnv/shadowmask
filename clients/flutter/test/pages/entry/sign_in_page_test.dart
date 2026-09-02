import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/testing.dart';
import 'package:http/http.dart' as http;
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/brand_mark.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/entry/sign_in_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

ApiClient _client(MockClient http) =>
    ApiClient(baseUrl: 'http://test', httpClient: http);

Widget _host(Widget child) => ThemeScope(
  variant: AppThemeVariant.dark,
  setVariant: (_) {},
  child: MaterialApp(theme: buildTheme(AppThemeVariant.dark), home: child),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('sign in requires a username and password', (
    WidgetTester tester,
  ) async {
    final ApiClient api = _client(
      MockClient((http.Request _) async => http.Response('', 401)),
    );
    await tester.pumpWidget(_host(SignInPage(api: api)));
    await tester.pumpAndSettle();

    expect(find.text(Strings.appName), findsNothing);
    expect(find.byType(BrandMark), findsOneWidget);
    expect(find.text(Strings.usernameAndPasswordRequired), findsNothing);

    await tester.tap(find.byType(FilledButton));
    await tester.pump();

    expect(find.text(Strings.usernameAndPasswordRequired), findsOneWidget);
  });

  testWidgets('the credentials are offered to the browser password manager', (
    WidgetTester tester,
  ) async {
    final ApiClient api = _client(
      MockClient((http.Request _) async => http.Response('', 401)),
    );
    await tester.pumpWidget(_host(SignInPage(api: api)));
    await tester.pumpAndSettle();

    expect(
      find.byType(AutofillGroup),
      findsOneWidget,
      reason: 'without a group the browser is never told the entry finished',
    );
    final List<TextField> fields = tester
        .widgetList<TextField>(find.byType(TextField))
        .toList();
    expect(fields.first.autofillHints, <String>[AutofillHints.username]);
    expect(fields.last.autofillHints, <String>[AutofillHints.password]);
  });

  testWidgets(
    'enter on the username moves to the password, it does not submit',
    (WidgetTester tester) async {
      final ApiClient api = _client(
        MockClient((http.Request _) async => http.Response('', 401)),
      );
      await tester.pumpWidget(_host(SignInPage(api: api)));
      await tester.pumpAndSettle();

      await tester.enterText(find.byType(TextField).first, 'pat');
      await tester.testTextInput.receiveAction(TextInputAction.next);
      await tester.pump();

      expect(
        find.text(Strings.usernameAndPasswordRequired),
        findsNothing,
        reason: 'submitting from the username field only ever fails',
      );
      final TextField password = tester.widget<TextField>(
        find.byType(TextField).last,
      );
      expect(password.focusNode?.hasFocus, isTrue);
    },
  );
}
