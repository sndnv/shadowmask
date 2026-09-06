import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/server_scope.dart';
import 'package:shadowmask/api/server_store.dart';
import 'package:shadowmask/api/token_store.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/tooltip_dismisser.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/main.dart';
import 'package:shadowmask/model/auth/auth_tokens.dart';
import 'package:shadowmask/pages/entry/server_page.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/theme_store.dart';
import 'package:shared_preferences/shared_preferences.dart';

ApiClient Function(String?) _factory() =>
    (String? baseUrl) => ApiClient(
      baseUrl: baseUrl ?? 'http://test',
      httpClient: MockClient(
        (http.Request _) async => http.Response('{}', 200),
      ),
    );

ThemeScope _theme(WidgetTester tester) =>
    tester.widget<ThemeScope>(find.byType(ThemeScope));

Future<void> _pumpConfigured(
  WidgetTester tester, {
  AppThemeVariant variant = AppThemeVariant.dark,
  bool? highContrast,
}) async {
  await tester.pumpWidget(
    ShadowmaskApp(
      initialServer: 'http://test',
      clientFactory: _factory(),
      initialVariant: variant,
      initialHighContrast: highContrast,
    ),
  );
  await tester.pump();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the toast host sits above the navigator', (
    WidgetTester tester,
  ) async {
    await _pumpConfigured(tester);

    final Finder host = find.byType(ToastHost);
    expect(host, findsOneWidget);
    expect(
      find.descendant(of: host, matching: find.byType(Navigator)),
      findsWidgets,
    );
  });

  testWidgets('a configured server boots past the entry screen', (
    WidgetTester tester,
  ) async {
    await _pumpConfigured(tester);
    expect(find.byType(ServerPage), findsNothing);
  });

  testWidgets('every screen sits under the tooltip dismisser', (
    WidgetTester tester,
  ) async {
    await _pumpConfigured(tester);

    expect(
      find.ancestor(
        of: find.byType(ToastHost),
        matching: find.byType(TooltipDismisser),
      ),
      findsOneWidget,
      reason:
          'a tooltip left up across a window resize trips a framework layout '
          'assertion, so the dismisser has to wrap the whole app',
    );
  });

  testWidgets('a device with no server address is asked for one first', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(ShadowmaskApp(clientFactory: _factory()));
    await tester.pumpAndSettle();

    expect(find.byType(ServerPage), findsOneWidget);
    expect(find.byType(ToastHost), findsOneWidget);
    expect(find.byType(TooltipDismisser), findsOneWidget);
  });

  testWidgets('accepting a server stores it and leaves the entry screen', (
    WidgetTester tester,
  ) async {
    await TokenStore().save(
      const AuthTokens(accessToken: 'a', refreshToken: 'r'),
    );
    await tester.pumpWidget(
      ShadowmaskApp(clientFactory: _factory(), probe: (_) async => true),
    );
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), 'http://two');
    await tester.tap(find.text(Strings.connect));
    await tester.pumpAndSettle();

    expect(await const ServerStore().load(), 'http://two');
    expect(find.byType(ServerPage), findsNothing);
    expect(
      await TokenStore().load(),
      isNull,
      reason:
          'credentials for the previous server must not follow the viewer to '
          'a different one',
    );
  });

  testWidgets('an address nothing answers on is refused and not stored', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      ShadowmaskApp(clientFactory: _factory(), probe: (_) async => false),
    );
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), 'http://nope');
    await tester.tap(find.text(Strings.connect));
    await tester.pumpAndSettle();

    expect(find.text(Strings.serverAddressUnanswered), findsOneWidget);
    expect(find.byType(ServerPage), findsOneWidget);
    expect(await const ServerStore().load(), isNull);
  });

  testWidgets('the server address is published to the app', (
    WidgetTester tester,
  ) async {
    await _pumpConfigured(tester);

    expect(
      tester.widget<ServerScope>(find.byType(ServerScope)).address,
      'http://test',
    );
  });

  testWidgets('the starting theme is published', (WidgetTester tester) async {
    await _pumpConfigured(tester, variant: AppThemeVariant.retro);
    expect(_theme(tester).variant, AppThemeVariant.retro);
  });

  testWidgets('switching the theme repaints and is written to the store', (
    WidgetTester tester,
  ) async {
    await _pumpConfigured(tester);

    _theme(tester).setVariant(AppThemeVariant.light);
    await tester.pumpAndSettle();

    expect(_theme(tester).variant, AppThemeVariant.light);
    expect(await const ThemeStore().load(), AppThemeVariant.light);
  });

  testWidgets('choosing the theme already in use writes nothing', (
    WidgetTester tester,
  ) async {
    await const ThemeStore().save(AppThemeVariant.retro);
    await _pumpConfigured(tester, variant: AppThemeVariant.light);

    _theme(tester).setVariant(AppThemeVariant.light);
    await tester.pumpAndSettle();

    expect(
      await const ThemeStore().load(),
      AppThemeVariant.retro,
      reason: 'a no-op choice must not overwrite what is on disk',
    );
  });

  testWidgets('high contrast can be turned on and is remembered', (
    WidgetTester tester,
  ) async {
    await _pumpConfigured(tester, highContrast: false);
    expect(_theme(tester).highContrast, isFalse);

    _theme(tester).setHighContrast(true);
    await tester.pumpAndSettle();

    expect(_theme(tester).highContrast, isTrue);
    expect(await const ThemeStore().loadHighContrast(), isTrue);
  });

  testWidgets('asking again for the contrast already set writes nothing', (
    WidgetTester tester,
  ) async {
    await _pumpConfigured(tester, highContrast: true);

    _theme(tester).setHighContrast(true);
    await tester.pumpAndSettle();

    expect(
      await const ThemeStore().loadHighContrast(),
      isNull,
      reason: 'a no-op choice must not write anything at all',
    );
  });

  testWidgets('with no stored choice the system setting decides contrast', (
    WidgetTester tester,
  ) async {
    await _pumpConfigured(tester);

    expect(
      _theme(tester).highContrast,
      isFalse,
      reason: 'the test platform reports no accessibility preference',
    );
  });

  testWidgets('an explicit contrast choice outranks the system setting', (
    WidgetTester tester,
  ) async {
    await _pumpConfigured(tester, highContrast: true);
    expect(_theme(tester).highContrast, isTrue);
  });

  testWidgets('turning on system contrast reaches an undecided app', (
    WidgetTester tester,
  ) async {
    addTearDown(tester.platformDispatcher.clearAccessibilityFeaturesTestValue);
    await _pumpConfigured(tester);
    expect(_theme(tester).highContrast, isFalse);

    tester.platformDispatcher.accessibilityFeaturesTestValue =
        const FakeAccessibilityFeatures(highContrast: true);
    await tester.pumpAndSettle();

    expect(_theme(tester).highContrast, isTrue);
  });

  testWidgets('a system contrast change that changes nothing is ignored', (
    WidgetTester tester,
  ) async {
    addTearDown(tester.platformDispatcher.clearAccessibilityFeaturesTestValue);
    await _pumpConfigured(tester);

    tester.platformDispatcher.accessibilityFeaturesTestValue =
        const FakeAccessibilityFeatures();
    await tester.pumpAndSettle();

    expect(_theme(tester).highContrast, isFalse);
  });

  testWidgets('a system contrast change cannot override an explicit choice', (
    WidgetTester tester,
  ) async {
    addTearDown(tester.platformDispatcher.clearAccessibilityFeaturesTestValue);
    await _pumpConfigured(tester, highContrast: false);

    tester.platformDispatcher.accessibilityFeaturesTestValue =
        const FakeAccessibilityFeatures(highContrast: true);
    await tester.pumpAndSettle();

    expect(
      _theme(tester).highContrast,
      isFalse,
      reason: 'the viewer said no, and the platform does not get a second vote',
    );
  });

  testWidgets('a deep link the router does not know falls back to the root', (
    WidgetTester tester,
  ) async {
    addTearDown(tester.platformDispatcher.clearDefaultRouteNameTestValue);
    tester.platformDispatcher.defaultRouteNameTestValue = '/no-such-place';

    await _pumpConfigured(tester);
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.byType(ToastHost), findsOneWidget);
  });

  testWidgets('without a factory the app builds its own client', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(const ShadowmaskApp(initialServer: 'http://test'));
    await tester.pump();

    expect(find.byType(ToastHost), findsOneWidget);
    expect(find.byType(ServerPage), findsNothing);
  });
}
