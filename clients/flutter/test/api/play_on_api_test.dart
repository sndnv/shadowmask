import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/api/play_on_api.dart';
import 'package:shadowmask/api/target_caller.dart';
import 'package:shadowmask/model/auth/device.dart';
import 'package:shadowmask/model/auth/play_command.dart';
import 'package:shadowmask/model/auth/play_on_device.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _android = <String, dynamic>{
  'kind': 'android',
  'addresses': <String>['10.0.0.5'],
  'port': 7777,
};

Map<String, dynamic> _device(String id, Map<String, dynamic>? target) =>
    <String, dynamic>{
      'id': id,
      'user_id': 'u1',
      'name': 'Device $id',
      'platform': 'android',
      'created_at': '2026-10-14T18:00:00Z',
      'target': target,
    };

const Version _version = Version(
  id: 'v1',
  title: TitleRef(type: TitleKind.movie, id: 'm1'),
  libraryId: 'l1',
  quality: Quality.fhd,
  container: 'mkv',
  durationMs: 600000,
  available: true,
);

http.Response _json(Object body, [int status = 200]) =>
    http.Response(jsonEncode(body), status);

http.Response _error(int status, String code) => _json(<String, dynamic>{
  'error': <String, dynamic>{'code': code, 'message': code},
}, status);

ApiClient _api(MockClient mock) =>
    ApiClient(baseUrl: 'http://test', httpClient: mock);

MockClient _targets(List<dynamic> own, {http.Response? shared}) => MockClient(
  (http.Request req) async => req.url.path == '/api/v1/devices/shared'
      ? shared ?? _json(<dynamic>[])
      : _json(own),
);

MockClient _devices() => _targets(<dynamic>[
  _device('d1', _android),
  _device('d2', null),
  _device('d3', _android),
]);

Map<String, dynamic> _shared(String id, Map<String, dynamic>? target) =>
    <String, dynamic>{
      'id': id,
      'name': 'Shared $id',
      'platform': 'roku',
      'owner': 'admin',
      'target': target,
    };

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('a native app can send', () {
    expect(playOnApi(_api(_devices())), isA<PlayOnApi>());
  });

  test(
    'targets leave out devices that never reported and this device',
    () async {
      SharedPreferences.setMockInitialValues(<String, Object>{
        'shadowmask.tokens': jsonEncode(<String, String>{
          'access_token': 'a',
          'device_id': 'd3',
        }),
      });

      final List<PlayOnDevice> found = await PlayOnApi(
        _api(_devices()),
      ).targets('u1');

      expect(found.map((PlayOnDevice d) => d.id), <String>['d1']);
    },
  );

  test('a password login sees every device that reported', () async {
    final List<PlayOnDevice> found = await PlayOnApi(
      _api(_devices()),
    ).targets('u1');

    expect(found.map((PlayOnDevice d) => d.id), <String>['d1', 'd3']);
    expect(found.map((PlayOnDevice d) => d.owner), <String?>[null, null]);
  });

  test('a device of a kind this app does not know is left out', () async {
    final Map<String, dynamic> television = <String, dynamic>{
      ..._android,
      'kind': 'television',
    };
    final List<PlayOnDevice> found = await PlayOnApi(
      _api(
        _targets(
          <dynamic>[_device('d1', television), _device('d2', _android)],
          shared: _json(<dynamic>[
            _shared('s1', television),
            _shared('s2', _android),
          ]),
        ),
      ),
    ).targets('u1');

    expect(found.map((PlayOnDevice d) => d.id), <String>['d2', 's2']);
  });

  test('shared devices follow the own ones, each with its owner', () async {
    final List<PlayOnDevice> found = await PlayOnApi(
      _api(
        _targets(
          <dynamic>[_device('d1', _android)],
          shared: _json(<dynamic>[
            _shared('s1', _android),
            _shared('s2', null),
          ]),
        ),
      ),
    ).targets('u1');

    expect(found.map((PlayOnDevice d) => d.id), <String>['d1', 's1']);
    expect(found.last.owner, 'admin');
    expect(found.last.name, 'Shared s1');
    expect(found.last.target?.port, 7777);
  });

  test('a failed shared list leaves the own devices', () async {
    final List<PlayOnDevice> found = await PlayOnApi(
      _api(
        _targets(<dynamic>[
          _device('d1', _android),
        ], shared: _error(404, 'not_found')),
      ),
    ).targets('u1');

    expect(found.map((PlayOnDevice d) => d.id), <String>['d1']);
  });

  test('targets read the user devices and the shared ones', () async {
    final List<http.Request> seen = <http.Request>[];
    await PlayOnApi(
      _api(
        MockClient((http.Request req) async {
          seen.add(req);
          return _json(<dynamic>[]);
        }),
      ),
    ).targets('u 1');

    expect(seen.map((http.Request r) => r.method), <String>['GET', 'GET']);
    expect(seen.map((http.Request r) => r.url.path), <String>[
      '/api/v1/users/u%201/devices',
      '/api/v1/devices/shared',
    ]);
  });

  test('mint posts the version to the device and reads the command', () async {
    late http.Request seen;
    final PlayCommand command = await PlayOnApi(
      _api(
        MockClient((http.Request req) async {
          seen = req;
          return _json(<String, dynamic>{
            'token': 'tok',
            'expires_at': '2026-10-14T18:03:11Z',
            'target': _android,
          });
        }),
      ),
    ).mint('d1', 'v1');

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/devices/d1/commands');
    expect(jsonDecode(seen.body), <String, dynamic>{'version_id': 'v1'});
    expect(command.token, 'tok');
    expect(command.target.port, 7777);
  });

  for (final (int status, String code) in <(int, String)>[
    (404, 'version_not_found'),
    (404, 'not_found'),
    (409, 'not_a_target'),
  ]) {
    test('mint surfaces [$code]', () async {
      final PlayOnApi playOn = PlayOnApi(
        _api(MockClient((http.Request req) async => _error(status, code))),
      );

      await expectLater(
        playOn.mint('d1', 'v1'),
        throwsA(
          isA<ApiException>()
              .having((ApiException e) => e.status, 'status', status)
              .having((ApiException e) => e.code, 'code', code),
        ),
      );
    });
  }

  test('send mints a command and hands it to the target', () async {
    final List<Uri> lan = <Uri>[];
    final PlayOnApi playOn = PlayOnApi(
      _api(
        MockClient(
          (http.Request req) async => _json(<String, dynamic>{
            'token': 'tok',
            'expires_at': '2026-10-14T18:03:11Z',
            'target': _android,
          }),
        ),
      ),
      caller: TargetCaller(
        client: MockClient((http.Request req) async {
          lan.add(req.url);
          return http.Response('', 202);
        }),
      ),
    );

    final PlayOnOutcome outcome = await playOn.send(
      PlayOnDevice.own(Device.fromJson(_device('d1', _android))),
      _version,
    );

    expect(outcome, PlayOnOutcome.playing);
    expect(lan, <Uri>[Uri.parse('http://10.0.0.5:7777/play')]);
  });

  test('a refused mint calls no target', () async {
    final List<Uri> lan = <Uri>[];
    final PlayOnApi playOn = PlayOnApi(
      _api(MockClient((http.Request req) async => _error(409, 'not_a_target'))),
      caller: TargetCaller(
        client: MockClient((http.Request req) async {
          lan.add(req.url);
          return http.Response('', 202);
        }),
      ),
    );

    await expectLater(
      playOn.send(
        PlayOnDevice.own(Device.fromJson(_device('d1', _android))),
        _version,
      ),
      throwsA(isA<ApiException>()),
    );
    expect(lan, isEmpty);
  });

  test('reportTarget puts this device as an android target', () async {
    late http.Request seen;
    await PlayOnApi(
      _api(
        MockClient((http.Request req) async {
          seen = req;
          return http.Response('', 204);
        }),
      ),
    ).reportTarget(<String>['192.168.1.20'], 41000);

    expect(seen.method, 'PUT');
    expect(seen.url.path, '/api/v1/devices/self/target');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'kind': 'android',
      'addresses': <String>['192.168.1.20'],
      'port': 41000,
    });
  });

  test('clearTarget deletes the report', () async {
    late http.Request seen;
    await PlayOnApi(
      _api(
        MockClient((http.Request req) async {
          seen = req;
          return http.Response('', 204);
        }),
      ),
    ).clearTarget();

    expect(seen.method, 'DELETE');
    expect(seen.url.path, '/api/v1/devices/self/target');
  });

  test('redeem posts the token and reads the version', () async {
    late http.Request seen;
    final String version = await PlayOnApi(
      _api(
        MockClient((http.Request req) async {
          seen = req;
          return _json(<String, String>{'version_id': 'v9'});
        }),
      ),
    ).redeem('tok');

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/devices/self/commands/redeem');
    expect(jsonDecode(seen.body), <String, String>{'token': 'tok'});
    expect(version, 'v9');
  });
}
