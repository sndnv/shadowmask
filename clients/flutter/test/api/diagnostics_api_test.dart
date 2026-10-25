import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/diagnostics_api.dart';
import 'package:shadowmask/api/token_store.dart';
import 'package:shadowmask/model/auth/auth_tokens.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('a report is POSTed with the player id and answers its id', () async {
    await TokenStore().save(
      const AuthTokens(accessToken: 'a1', refreshToken: 'r1', playerId: 'p-9'),
    );
    late http.Request seen;
    final DiagnosticsApi diagnostics = DiagnosticsApi(
      ApiClient(
        baseUrl: 'http://test',
        httpClient: MockClient((http.Request req) async {
          seen = req;
          return http.Response(jsonEncode(<String, String>{'id': 'r1'}), 201);
        }),
      ),
    );

    final String id = await diagnostics.send(
      app: 'flutter',
      platform: 'android 14',
      appVersion: '0.0.7',
      note: 'stutters',
      payload: <String, Object?>{'schema': 1},
    );

    expect(id, 'r1');
    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/diagnostics/reports');
    expect(seen.headers['Authorization'], 'Bearer a1');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'player_id': 'p-9',
      'app': 'flutter',
      'platform': 'android 14',
      'app_version': '0.0.7',
      'note': 'stutters',
      'payload': <String, dynamic>{'schema': 1},
    });
  });
}
