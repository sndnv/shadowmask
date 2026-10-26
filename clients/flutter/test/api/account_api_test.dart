import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/account_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shared_preferences/shared_preferences.dart';

AccountApi _account(MockClient mock) =>
    AccountApi(ApiClient(baseUrl: 'http://test', httpClient: mock));

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('changePassword sends current and new passwords', () async {
    late http.Request seen;
    final AccountApi account = _account(
      MockClient((http.Request req) async {
        seen = req;
        return http.Response('', 204);
      }),
    );

    await account.changePassword('u1', current: 'old', newPassword: 'new');

    expect(seen.method, 'PUT');
    expect(seen.url.path, '/api/v1/users/u1/password');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'current_password': 'old',
      'new_password': 'new',
    });
  });

  test('changePassword omits an empty current password', () async {
    late http.Request seen;
    final AccountApi account = _account(
      MockClient((http.Request req) async {
        seen = req;
        return http.Response('', 204);
      }),
    );

    await account.changePassword('u1', newPassword: 'new');

    expect(jsonDecode(seen.body), <String, dynamic>{'new_password': 'new'});
  });

  test('signOutEverywhere deletes the sessions collection', () async {
    late http.Request seen;
    final AccountApi account = _account(
      MockClient((http.Request req) async {
        seen = req;
        return http.Response('', 204);
      }),
    );

    await account.signOutEverywhere('u1');

    expect(seen.method, 'DELETE');
    expect(seen.url.path, '/api/v1/users/u1/sessions');
  });

  test('shareDevice puts the username on the device', () async {
    late http.Request seen;
    final AccountApi account = _account(
      MockClient((http.Request req) async {
        seen = req;
        return http.Response('{"user_id":"u2","username":"kid"}', 200);
      }),
    );

    await account.shareDevice('d 1', 'kid');

    expect(seen.method, 'PUT');
    expect(seen.url.path, '/api/v1/devices/d%201/shares');
    expect(jsonDecode(seen.body), <String, dynamic>{'username': 'kid'});
  });

  test('unshareDevice deletes the user from the device', () async {
    late http.Request seen;
    final AccountApi account = _account(
      MockClient((http.Request req) async {
        seen = req;
        return http.Response('', 204);
      }),
    );

    await account.unshareDevice('d1', 'u 2');

    expect(seen.method, 'DELETE');
    expect(seen.url.path, '/api/v1/devices/d1/shares/u%202');
  });

  test('devices carry whom each is shared with', () async {
    final AccountApi account = _account(
      MockClient(
        (http.Request req) async => http.Response(
          jsonEncode(<dynamic>[
            <String, dynamic>{
              'id': 'd1',
              'user_id': 'u1',
              'name': 'Roku',
              'platform': 'roku',
              'created_at': '2026-08-17T09:00:00Z',
              'shared_with': <dynamic>[
                <String, dynamic>{'user_id': 'u2', 'username': 'kid'},
              ],
            },
            <String, dynamic>{
              'id': 'd2',
              'user_id': 'u1',
              'name': 'Pixel',
              'platform': 'android',
              'created_at': '2026-08-17T09:00:00Z',
            },
          ]),
          200,
        ),
      ),
    );

    final devices = await account.devices('u1');

    expect(devices.first.sharedWith.single.username, 'kid');
    expect(devices.first.sharedWith.single.userId, 'u2');
    expect(devices.last.sharedWith, isEmpty);
  });

  test('createLinkCode posts the user id and parses the code', () async {
    Map<String, dynamic>? body;
    final AccountApi account = _account(
      MockClient((http.Request req) async {
        body = jsonDecode(req.body) as Map<String, dynamic>;
        return http.Response(
          jsonEncode(<String, dynamic>{
            'code': 'ABCD',
            'expires_at': '2026-08-13T00:00:00Z',
          }),
          200,
        );
      }),
    );

    final code = await account.createLinkCode('u1');

    expect(body!['user_id'], 'u1');
    expect(code.code, 'ABCD');
    expect(code.expiresAt, '2026-08-13T00:00:00Z');
  });
}
