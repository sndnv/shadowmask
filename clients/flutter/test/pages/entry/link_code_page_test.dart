import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/entry/link_code_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Widget _host(Widget child) => ThemeScope(
  variant: AppThemeVariant.dark,
  setVariant: (_) {},
  child: MaterialApp(
    theme: buildTheme(AppThemeVariant.dark),
    initialRoute: '/link',
    routes: <String, WidgetBuilder>{
      '/': (BuildContext context) => const Text('sign in'),
      '/home': (BuildContext context) => const Text('home'),
      '/link': (BuildContext context) => child,
    },
  ),
);

Finder _field(String label) =>
    find.ancestor(of: find.text(label), matching: find.byType(TextField));

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a code and a device name are both required', (
    WidgetTester tester,
  ) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request _) async => http.Response('', 500)),
    );
    await tester.pumpWidget(
      _host(LinkCodePage(api: api, defaultDeviceName: () async => 'Pixel 7')),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byType(FilledButton));
    await tester.pump();
    expect(find.text(Strings.linkCodeRequired), findsOneWidget);

    await tester.enterText(_field(Strings.linkCodeLabel), '7G2K9QMP');
    await tester.enterText(_field(Strings.deviceNameLabel), '   ');
    await tester.tap(find.byType(FilledButton));
    await tester.pump();
    expect(find.text(Strings.deviceNameRequired), findsOneWidget);
  });

  testWidgets('the device name is prefilled from the device itself', (
    WidgetTester tester,
  ) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request _) async => http.Response('', 500)),
    );
    await tester.pumpWidget(
      _host(LinkCodePage(api: api, defaultDeviceName: () async => 'Pixel 7')),
    );
    await tester.pumpAndSettle();

    expect(find.text('Pixel 7'), findsOneWidget);
  });

  testWidgets('a device with no name of its own still gets a usable one', (
    WidgetTester tester,
  ) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request _) async => http.Response('', 500)),
    );
    await tester.pumpWidget(
      _host(LinkCodePage(api: api, defaultDeviceName: () async => null)),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.deviceNameFallback), findsOneWidget);
  });

  testWidgets('the typed code is grouped and folded to upper case', (
    WidgetTester tester,
  ) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request _) async => http.Response('', 500)),
    );
    await tester.pumpWidget(
      _host(LinkCodePage(api: api, defaultDeviceName: () async => null)),
    );
    await tester.pumpAndSettle();

    await tester.enterText(_field(Strings.linkCodeLabel), '7g2k9qmp');
    await tester.pump();

    expect(find.text('7G2K 9QMP'), findsOneWidget);
  });

  testWidgets('redeeming binds the device and lands on the home screen', (
    WidgetTester tester,
  ) async {
    final List<http.Request> sent = <http.Request>[];
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request request) async {
        sent.add(request);
        if (request.url.path == '/api/v1/auth/link') {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'token': 'smk_abc',
              'expires_at': null,
            }),
            200,
          );
        }
        return http.Response('', 404);
      }),
    );
    await tester.pumpWidget(
      _host(
        LinkCodePage(api: api, defaultDeviceName: () async => 'Living room'),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(_field(Strings.linkCodeLabel), '7G2K9QMP');
    await tester.tap(find.byType(FilledButton));
    await tester.pumpAndSettle();

    expect(find.text('home'), findsOneWidget);

    final Map<String, dynamic> body =
        jsonDecode(sent.single.body) as Map<String, dynamic>;
    expect(
      body['code'],
      '7G2K9QMP',
      reason: 'the grouping is for reading, it never reaches the wire',
    );
    final Map<String, dynamic> device = body['device'] as Map<String, dynamic>;
    expect(device['name'], 'Living room');
    expect(device['platform'], isNotEmpty);

    expect(
      sent.single.headers.containsKey('Authorization'),
      isFalse,
      reason: 'redeeming is the one call made before there is any token',
    );
  });

  testWidgets('a rejected code reports the failure and stays on the page', (
    WidgetTester tester,
  ) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient(
        (http.Request _) async => http.Response(
          jsonEncode(<String, dynamic>{
            'error': <String, dynamic>{'message': 'unknown link code'},
          }),
          404,
        ),
      ),
    );
    await tester.pumpWidget(
      _host(LinkCodePage(api: api, defaultDeviceName: () async => 'Phone')),
    );
    await tester.pumpAndSettle();

    await tester.enterText(_field(Strings.linkCodeLabel), 'BADCODE1');
    await tester.tap(find.byType(FilledButton));
    await tester.pumpAndSettle();

    expect(find.textContaining('unknown link code'), findsOneWidget);
    expect(find.text('home'), findsNothing);
  });

  testWidgets('a password sign in is always one tap away', (
    WidgetTester tester,
  ) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request _) async => http.Response('', 500)),
    );
    await tester.pumpWidget(
      _host(LinkCodePage(api: api, defaultDeviceName: () async => null)),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.usePassword));
    await tester.pumpAndSettle();

    expect(find.text('sign in'), findsOneWidget);
  });
}
