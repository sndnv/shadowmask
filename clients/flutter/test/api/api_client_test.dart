import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/api/authentication_failure.dart';
import 'package:shadowmask/api/authorization_failure.dart';
import 'package:shadowmask/api/token_store.dart';
import 'package:shadowmask/model/auth/auth_tokens.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shared_preferences/shared_preferences.dart';

ApiClient _client(MockClient mock) =>
    ApiClient(baseUrl: 'http://test', httpClient: mock);

http.Response _pair(String access, String refresh) => http.Response(
  jsonEncode(<String, String>{
    'access_token': access,
    'refresh_token': refresh,
  }),
  200,
);

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

  test('requests refused together share one refresh', () async {
    int refreshes = 0;
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        if (req.url.path == '/api/v1/auth/login') {
          return _pair('stale', 'r1');
        }
        if (req.url.path == '/api/v1/auth/refresh') {
          refreshes += 1;
          await Future<void>.delayed(const Duration(milliseconds: 10));
          return refreshes == 1 ? _pair('fresh', 'r2') : http.Response('', 401);
        }
        final bool fresh = req.headers['Authorization'] == 'Bearer fresh';
        return http.Response('', fresh ? 204 : 401);
      }),
    );
    await api.login('ada', 'pw');

    final List<int> statuses = await Future.wait(<Future<int>>[
      for (int row = 0; row < 4; row++)
        api.sendStatus('GET', '/api/v1/rows/$row'),
    ]);

    expect(statuses, <int>[204, 204, 204, 204]);
    expect(
      refreshes,
      1,
      reason: 'a second refresh spends a token the first one revoked',
    );
    expect((await api.currentTokens())?.refreshToken, 'r2');
  });

  test('tokens another tab refreshed are used without refreshing', () async {
    final List<String> paths = <String>[];
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        paths.add(req.url.path);
        if (req.url.path == '/api/v1/auth/login') {
          return _pair('stale', 'r1');
        }
        if (req.headers['Authorization'] == 'Bearer fresh') {
          return http.Response('', 204);
        }
        await TokenStore().save(
          const AuthTokens(accessToken: 'fresh', refreshToken: 'r2'),
        );
        return http.Response('', 401);
      }),
    );
    await api.login('ada', 'pw');

    expect(await api.sendStatus('GET', '/api/v1/rows'), 204);
    expect(paths, isNot(contains('/api/v1/auth/refresh')));
  });

  test('a refused refresh signs out', () async {
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        if (req.url.path == '/api/v1/auth/login') {
          return _pair('stale', 'r1');
        }
        return http.Response('', 401);
      }),
    );
    await api.login('ada', 'pw');

    await expectLater(
      api.sendStatus('GET', '/api/v1/rows'),
      throwsA(isA<AuthenticationFailure>()),
    );
    expect(await api.currentTokens(), isNull);
  });

  for (final int status in <int>[404, 408, 429, 502]) {
    test('a refresh answered $status keeps the sign-in', () async {
      final ApiClient api = _client(
        MockClient((http.Request req) async {
          if (req.url.path == '/api/v1/auth/login') {
            return _pair('stale', 'r1');
          }
          if (req.url.path == '/api/v1/auth/refresh') {
            return http.Response('', status);
          }
          return http.Response('', 401);
        }),
      );
      await api.login('ada', 'pw');

      await expectLater(
        api.sendStatus('GET', '/api/v1/rows'),
        throwsA(
          isA<ApiException>()
              .having((ApiException e) => e.status, 'status', status)
              .having(
                (ApiException e) => e,
                'kind',
                isNot(isA<AuthenticationFailure>()),
              ),
        ),
      );
      expect((await api.currentTokens())?.refreshToken, 'r1');
    });
  }

  test('a refresh lost on the network keeps the sign-in', () async {
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        if (req.url.path == '/api/v1/auth/login') {
          return _pair('stale', 'r1');
        }
        if (req.url.path == '/api/v1/auth/refresh') {
          throw http.ClientException('connection refused');
        }
        return http.Response('', 401);
      }),
    );
    await api.login('ada', 'pw');

    await expectLater(
      api.sendStatus('GET', '/api/v1/rows'),
      throwsA(isA<http.ClientException>()),
    );
    expect((await api.currentTokens())?.refreshToken, 'r1');
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

  test('a public read sends no token and returns the body', () async {
    final List<http.Request> seen = <http.Request>[];
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        seen.add(req);
        if (req.url.path == '/api/v1/auth/login') {
          return http.Response(
            jsonEncode(<String, String>{
              'access_token': 'a1',
              'refresh_token': 'r1',
            }),
            200,
          );
        }
        return http.Response('up 1\n', 200);
      }),
    );
    await api.login('ada', 'pw');

    expect(await api.publicText('/metrics'), 'up 1\n');
    expect(seen.last.url.path, '/metrics');
    expect(seen.last.headers.containsKey('Authorization'), isFalse);
  });

  test('a refused public read is an error and keeps the session', () async {
    final List<String> paths = <String>[];
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        paths.add(req.url.path);
        if (req.url.path == '/api/v1/auth/login') {
          return http.Response(
            jsonEncode(<String, String>{
              'access_token': 'a1',
              'refresh_token': 'r1',
            }),
            200,
          );
        }
        return http.Response('Unauthorized', 401);
      }),
    );
    await api.login('ada', 'pw');

    await expectLater(
      api.publicText('/metrics'),
      throwsA(
        isA<ApiException>()
            .having((ApiException e) => e.status, 'status', 401)
            .having(
              (ApiException e) => e,
              'kind',
              isNot(isA<AuthenticationFailure>()),
            ),
      ),
    );
    expect(paths, isNot(contains('/api/v1/auth/refresh')));
    expect((await api.currentTokens())?.accessToken, 'a1');
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

  test('a redeemed link code remembers which device row it created', () async {
    final ApiClient api = _client(
      MockClient(
        (http.Request _) async => http.Response(
          jsonEncode(<String, dynamic>{
            'token': 'smk_abc',
            'expires_at': null,
            'device_id': 'dev-7',
          }),
          200,
        ),
      ),
    );

    await api.redeemLinkCode(
      '7G2K9QMP',
      deviceName: 'Kitchen tablet',
      platform: 'android',
    );

    expect(await api.linkedDeviceId(), 'dev-7');
  });

  test(
    'a password login names no device, so there is none to revoke',
    () async {
      final ApiClient api = _client(
        MockClient(
          (http.Request _) async => http.Response(
            jsonEncode(<String, String>{
              'access_token': 'a',
              'refresh_token': 'r',
            }),
            200,
          ),
        ),
      );

      await api.login('u', 'p');

      expect(await api.linkedDeviceId(), isEmpty);
    },
  );

  test('signing out forgets the device id along with the token', () async {
    final ApiClient api = _client(
      MockClient(
        (http.Request _) async => http.Response(
          jsonEncode(<String, dynamic>{
            'token': 'smk_abc',
            'device_id': 'dev-7',
          }),
          200,
        ),
      ),
    );

    await api.redeemLinkCode('7G2K9QMP', deviceName: 'd', platform: 'android');
    await api.logout();

    expect(await api.linkedDeviceId(), isEmpty);
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

  test('a login, a refresh and a sign-out each announce a change', () async {
    final ApiClient api = _client(
      MockClient((http.Request req) async {
        if (req.url.path == '/api/v1/auth/login') {
          return _pair('stale', 'r1');
        }
        if (req.url.path == '/api/v1/auth/refresh') {
          return _pair('fresh', 'r2');
        }
        if (req.url.path == '/api/v1/auth/logout') {
          return http.Response('', 204);
        }
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
    int changes = 0;
    api.sessionChanged.listen((_) => changes += 1);

    await api.login('ada', 'pw');
    await pumpEventQueue();
    expect(changes, 1);

    await api.currentUser();
    await pumpEventQueue();
    expect(changes, 2);

    await api.logout();
    await pumpEventQueue();
    expect(changes, 3);
  });

  test(
    'a link and a 401 that signs the device out announce a change',
    () async {
      final ApiClient api = _client(
        MockClient(
          (http.Request req) async => req.url.path == '/api/v1/auth/link'
              ? http.Response(
                  jsonEncode(<String, dynamic>{
                    'token': 'smk_abc',
                    'device_id': 'dev-7',
                  }),
                  200,
                )
              : http.Response('', 401),
        ),
      );
      int changes = 0;
      api.sessionChanged.listen((_) => changes += 1);

      await api.redeemLinkCode(
        'C0DE',
        deviceName: 'Tablet',
        platform: 'android',
      );
      await pumpEventQueue();
      expect(changes, 1);

      await expectLater(
        api.currentUser(),
        throwsA(isA<AuthenticationFailure>()),
      );
      await pumpEventQueue();
      expect(changes, 2);
      expect(await api.linkedDeviceId(), isEmpty);
    },
  );

  group('player id', () {
    late int made;

    String nextId() => 'p-${++made}';

    setUp(() => made = 0);

    ApiClient playerClient({bool refreshes = false}) => ApiClient(
      baseUrl: 'http://test',
      newPlayerId: nextId,
      httpClient: MockClient((http.Request req) async {
        switch (req.url.path) {
          case '/api/v1/auth/login':
            return _pair('a1', 'r1');
          case '/api/v1/auth/refresh':
            return _pair('a2', 'r2');
          case '/api/v1/auth/link':
            return http.Response(
              jsonEncode(<String, dynamic>{
                'token': 'smk_abc',
                'device_id': 'dev-7',
              }),
              200,
            );
        }
        final bool fresh = req.headers['Authorization'] == 'Bearer a2';
        return refreshes && !fresh
            ? http.Response('', 401)
            : http.Response(
                jsonEncode(<String, String>{
                  'id': 'u1',
                  'username': 'ada',
                  'role': 'user',
                }),
                200,
              );
      }),
    );

    test('a password sign-in gets one, kept with its tokens', () async {
      final ApiClient api = playerClient();

      await api.login('ada', 'pw');

      expect(await api.playerId(), 'p-1');
      expect(await api.playerId(), 'p-1');
      expect((await TokenStore().load())!.playerId, 'p-1');
      expect(made, 1);
    });

    test('a refresh keeps it', () async {
      final ApiClient api = playerClient(refreshes: true);
      await api.login('ada', 'pw');

      await api.currentUser();

      final AuthTokens stored = (await TokenStore().load())!;
      expect(stored.accessToken, 'a2', reason: 'the refresh did happen');
      expect(stored.playerId, 'p-1');
      expect(made, 1);
    });

    test('a sign-out ends it and the next sign-in gets a new one', () async {
      final ApiClient api = playerClient();
      await api.login('ada', 'pw');

      await api.logout();
      expect(await api.playerId(), isEmpty);

      await api.login('bob', 'pw');
      expect(await api.playerId(), 'p-2');
    });

    test('a linked app is its own player and makes none', () async {
      final ApiClient api = playerClient();

      await api.redeemLinkCode('C0DE', deviceName: 'd', platform: 'android');

      expect(await api.playerId(), 'dev-7');
      expect(made, 0);
    });

    test(
      'a sign-in from before player ids gets one its next refresh stores',
      () async {
        await TokenStore().save(
          const AuthTokens(accessToken: 'a1', refreshToken: 'r1'),
        );
        final ApiClient api = playerClient(refreshes: true);

        expect(await api.playerId(), 'p-1');
        expect((await TokenStore().load())!.playerId, isEmpty);

        await api.currentUser();

        expect((await TokenStore().load())!.playerId, 'p-1');
        expect(await api.playerId(), 'p-1');
        expect(made, 1);
      },
    );
  });
}
