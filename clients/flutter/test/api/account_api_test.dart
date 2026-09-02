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
