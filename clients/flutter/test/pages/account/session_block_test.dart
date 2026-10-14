import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/play_on_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/session_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/play_on_receiver.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/play_on_target.dart';

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

Future<void> _signOutAndConfirm(WidgetTester tester) async {
  await tester.tap(find.text(Strings.signOut));
  await tester.pumpAndSettle();
  await tester.tap(find.widgetWithText(FilledButton, Strings.signOut));
  await tester.pumpAndSettle();
}

Future<(PlayOnReceiver, FakeBinder)> _listening(WidgetTester tester) async {
  SharedPreferences.setMockInitialValues(<String, Object>{
    'shadowmask.playOn.enabled': true,
  });
  final ApiClient api = await _linked(
    MockClient(
      (http.Request req) async => http.Response(
        jsonEncode(<String, dynamic>{'token': 'smk_abc', 'device_id': 'dev-7'}),
        200,
      ),
    ),
  );
  final FakeBinder binder = FakeBinder();
  final PlayOnReceiver receiver = PlayOnReceiver(
    api: api,
    onPlay: (String _) async {},
    onRefused: () {},
    bind: binder.call,
    addresses: () async => kWifi,
  );
  addTearDown(receiver.dispose);
  receiver.load();
  await tester.pumpWidget(PlayOnScope(receiver: receiver, child: _host(api)));
  await tester.pumpAndSettle();
  expect(binder.made, hasLength(1));
  return (receiver, binder);
}

ApiClient _recording(List<String> calls) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    calls.add('${req.method} ${req.url.path}');
    return http.Response('', 204);
  }),
);

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
    await _signOutAndConfirm(tester);

    expect(calls, contains('DELETE /api/v1/users/u1/devices/dev-7'));
    expect(await api.linkedDeviceId(), isEmpty);
  });

  testWidgets('signing out stops listening for other devices', (
    WidgetTester tester,
  ) async {
    final (PlayOnReceiver receiver, FakeBinder binder) = await _listening(
      tester,
    );

    await _signOutAndConfirm(tester);

    expect(receiver.linked, isFalse);
    expect(receiver.enabled, isFalse);
    expect(binder.last.closed, isTrue);
  });

  testWidgets('signing out everywhere stops listening for other devices', (
    WidgetTester tester,
  ) async {
    final (PlayOnReceiver receiver, FakeBinder binder) = await _listening(
      tester,
    );

    await tester.tap(find.text(Strings.signOutEverywhere));
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, Strings.signOutEverywhere).last,
    );
    await tester.pumpAndSettle();

    expect(receiver.linked, isFalse);
    expect(receiver.enabled, isFalse);
    expect(binder.last.closed, isTrue);
  });

  testWidgets('cancelling the sign out leaves the device signed in', (
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
    expect(find.text(Strings.confirmSignOut), findsOneWidget);
    await tester.tap(find.text(Strings.cancel));
    await tester.pumpAndSettle();

    expect(calls, isEmpty);
    expect(await api.linkedDeviceId(), 'dev-7');
    expect(find.byType(SessionBlock), findsOneWidget);
  });

  testWidgets('cancelling sign out everywhere revokes nothing', (
    WidgetTester tester,
  ) async {
    final List<String> calls = <String>[];

    await tester.pumpWidget(_host(_recording(calls)));
    await tester.tap(find.text(Strings.signOutEverywhere));
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmSignOutEverywhere), findsOneWidget);
    await tester.tap(find.text(Strings.cancel));
    await tester.pumpAndSettle();

    expect(calls, isEmpty);
    expect(find.byType(SessionBlock), findsOneWidget);
  });

  testWidgets('confirming sign out everywhere revokes every session', (
    WidgetTester tester,
  ) async {
    final List<String> calls = <String>[];

    await tester.pumpWidget(_host(_recording(calls)));
    await tester.tap(find.text(Strings.signOutEverywhere));
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, Strings.signOutEverywhere).last,
    );
    await tester.pumpAndSettle();

    expect(calls, contains('DELETE /api/v1/users/u1/sessions'));
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
    await _signOutAndConfirm(tester);

    expect(await api.linkedDeviceId(), isEmpty);
  });

  testWidgets('a device with no stored id skips the revoke entirely', (
    WidgetTester tester,
  ) async {
    final List<String> calls = <String>[];

    await tester.pumpWidget(_host(_recording(calls)));
    await _signOutAndConfirm(tester);

    expect(calls.where((String c) => c.contains('/devices/')), isEmpty);
  });
}
