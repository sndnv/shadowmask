import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/session_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

Widget _host(ApiClient api) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  routes: <String, WidgetBuilder>{
    '/': (_) => Scaffold(
      body: SingleChildScrollView(
        child: SessionBlock(api: api, userId: 'u1'),
      ),
    ),
  },
);

Future<ApiClient> _linked(MockClient mock) async {
  final ApiClient api = ApiClient(baseUrl: 'http://test', httpClient: mock);
  await api.redeemLinkCode('CODE', deviceName: 'd', platform: 'android');
  return api;
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('signing out revokes this device before it clears the token', (
    WidgetTester tester,
  ) async {
    final List<String> calls = <String>[];
    final ApiClient api = await _linked(
      MockClient((http.Request req) async {
        calls.add('${req.method} ${req.url.path}');
        return http.Response(
          jsonEncode(<String, dynamic>{
            'token': 'smk_abc',
            'device_id': 'dev-7',
          }),
          200,
        );
      }),
    );
    calls.clear();

    await tester.pumpWidget(_host(api));
    await tester.tap(find.text(Strings.signOut));
    await tester.pumpAndSettle();

    expect(calls, contains('DELETE /api/v1/users/u1/devices/dev-7'));
    expect(await api.linkedDeviceId(), isEmpty);
  });

  testWidgets('a server that refuses the revoke still signs the device out', (
    WidgetTester tester,
  ) async {
    bool linked = false;
    final ApiClient api = await _linked(
      MockClient((http.Request req) async {
        if (linked) {
          return http.Response('nope', 500);
        }
        linked = true;
        return http.Response(
          jsonEncode(<String, dynamic>{
            'token': 'smk_abc',
            'device_id': 'dev-7',
          }),
          200,
        );
      }),
    );

    await tester.pumpWidget(_host(api));
    await tester.tap(find.text(Strings.signOut));
    await tester.pumpAndSettle();

    expect(await api.linkedDeviceId(), isEmpty);
  });

  testWidgets('a device with no stored id skips the revoke entirely', (
    WidgetTester tester,
  ) async {
    final List<String> calls = <String>[];
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        calls.add('${req.method} ${req.url.path}');
        return http.Response('', 204);
      }),
    );

    await tester.pumpWidget(_host(api));
    await tester.tap(find.text(Strings.signOut));
    await tester.pumpAndSettle();

    expect(calls.where((String c) => c.contains('/devices/')), isEmpty);
  });
}
