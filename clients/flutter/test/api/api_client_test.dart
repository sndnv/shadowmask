import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/authentication_failure.dart';
import 'package:shadowmask/api/authorization_failure.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shared_preferences/shared_preferences.dart';

ApiClient _client(MockClient mock) =>
    ApiClient(baseUrl: 'http://test', httpClient: mock);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('login stores the token pair and authorizes later calls', () async {
    String? seenAuth;
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        if (req.url.path == '/api/v1/auth/login') {
          return http.Response(
            jsonEncode(<String, String>{
              'access_token': 'a1',
              'refresh_token': 'r1',
            }),
            200,
          );
        }
        seenAuth = req.headers['Authorization'];
        return http.Response(
          jsonEncode(<String, String>{
            'id': 'u1',
            'username': 'ada',
            'role': 'admin',
          }),
          200,
        );
      }),
    );

    await api.login('ada', 'pw');
    final SelfUser user = await api.currentUser();

    expect(seenAuth, 'Bearer a1');
    expect(user.username, 'ada');
    expect(user.isAdmin, isTrue);
  });

  test('the current user is fetched once and reused across pages', () async {
    int selfCalls = 0;
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        if (req.url.path == '/api/v1/auth/logout') {
          return http.Response('', 204);
        }
        selfCalls += 1;
        return http.Response(
          jsonEncode(<String, String>{
            'id': 'u1',
            'username': 'ada',
            'role': 'admin',
          }),
          200,
        );
      }),
    );

    await api.currentUser();
    await api.currentUser();
    await api.currentUser();
    expect(selfCalls, 1);

    await api.logout();
    await api.currentUser();
    expect(selfCalls, 2);
  });

  test('any write drops the cached user', () async {
    int selfCalls = 0;
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        if (req.method != 'GET') {
          return http.Response('', 204);
        }
        selfCalls += 1;
        return http.Response(
          jsonEncode(<String, String>{
            'id': 'u1',
            'username': 'ada',
            'role': 'user',
          }),
          200,
        );
      }),
    );

    await api.currentUser();
    await api.sendVoid('PUT', '/api/v1/users/u1');
    await api.currentUser();

    expect(selfCalls, 2);
  });

  test('a failed user fetch is not cached', () async {
    int selfCalls = 0;
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        selfCalls += 1;
        if (selfCalls == 1) {
          return http.Response('', 500);
        }
        return http.Response(
          jsonEncode(<String, String>{
            'id': 'u1',
            'username': 'ada',
            'role': 'user',
          }),
          200,
        );
      }),
    );

    await expectLater(api.currentUser(), throwsA(isA<Object>()));
    final SelfUser user = await api.currentUser();

    expect(selfCalls, 2);
    expect(user.username, 'ada');
  });

  test('a 401 triggers one refresh then retries with the new token', () async {
    int selfCalls = 0;
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        if (req.url.path == '/api/v1/auth/login') {
          return http.Response(
            jsonEncode(<String, String>{
              'access_token': 'stale',
              'refresh_token': 'r1',
            }),
            200,
          );
        }
        if (req.url.path == '/api/v1/auth/refresh') {
          return http.Response(
            jsonEncode(<String, String>{
              'access_token': 'fresh',
              'refresh_token': 'r2',
            }),
            200,
          );
        }
        selfCalls += 1;
        final bool fresh = req.headers['Authorization'] == 'Bearer fresh';
        return http.Response(
          fresh
              ? jsonEncode(<String, String>{
                  'id': 'u1',
                  'username': 'ada',
                  'role': 'user',
                })
              : '',
          fresh ? 200 : 401,
        );
      }),
    );

    await api.login('ada', 'pw');
    final SelfUser user = await api.currentUser();

    expect(user.role, UserRole.user);
    expect(selfCalls, 2);
  });

  test('403 maps to AuthorizationFailure', () async {
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        if (req.url.path == '/api/v1/auth/login') {
          return http.Response(
            jsonEncode(<String, String>{
              'access_token': 'a1',
              'refresh_token': 'r1',
            }),
            200,
          );
        }
        return http.Response('', 403);
      }),
    );
    await api.login('ada', 'pw');
    expect(api.currentUser(), throwsA(isA<AuthorizationFailure>()));
  });

  test('an unauthenticated call maps to AuthenticationFailure', () async {
    final ApiClient api = _client(
      MockClient((http.Request _) async => http.Response('', 401)),
    );
    expect(api.currentUser(), throwsA(isA<AuthenticationFailure>()));
  });

  test('a redeemed link code authorizes later calls', () async {
    String? seenAuth;
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        if (req.url.path == '/api/v1/auth/link') {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'token': 'smk_abc',
              'expires_at': null,
            }),
            200,
          );
        }
        seenAuth = req.headers['Authorization'];
        return http.Response(
          jsonEncode(<String, String>{
            'id': 'u1',
            'username': 'tablet',
            'role': 'player',
          }),
          200,
        );
      }),
    );

    await api.redeemLinkCode(
      '7G2K9QMP',
      deviceName: 'Kitchen tablet',
      platform: 'android',
    );
    final SelfUser user = await api.currentUser();

    expect(seenAuth, 'Bearer smk_abc');
    expect(user.role, UserRole.player);
  });

  test(
    'an api token is never sent for refresh, it just expires the session',
    () async {
      final List<String> paths = <String>[];
      final ApiClient api = _client(
        MockClient((http.Request req) async {
          paths.add(req.url.path);
          if (req.url.path == '/api/v1/auth/link') {
            return http.Response(
              jsonEncode(<String, dynamic>{'token': 'smk_abc'}),
              200,
            );
          }
          return http.Response('', 401);
        }),
      );

      await api.redeemLinkCode(
        'C0DE',
        deviceName: 'Tablet',
        platform: 'android',
      );

      await expectLater(
        api.currentUser(),
        throwsA(isA<AuthenticationFailure>()),
        reason: 'a revoked device should land on sign in, not loop',
      );
      expect(
        paths.contains('/api/v1/auth/refresh'),
        isFalse,
        reason: 'there is no refresh token to spend',
      );
    },
  );

  test('signing out an api token device is a local matter', () async {
    final List<String> paths = <String>[];
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        paths.add(req.url.path);
        return http.Response(
          jsonEncode(<String, dynamic>{'token': 'smk_abc'}),
          200,
        );
      }),
    );

    await api.redeemLinkCode('C0DE', deviceName: 'Tablet', platform: 'android');
    await api.logout();

    expect(paths, <String>['/api/v1/auth/link']);
    expect(await api.currentTokens(), isNull);
  });
}
