import 'dart:convert';

import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/play_on_api.dart';
import 'package:shadowmask/api/target_caller.dart';
import 'package:shadowmask/model/auth/device.dart';
import 'package:shadowmask/model/auth/device_target.dart';
import 'package:shadowmask/model/auth/play_on_device.dart';

const DeviceTarget kRokuTarget = DeviceTarget(
  kind: TargetKind.roku,
  addresses: <String>['10.0.0.2'],
  port: 8060,
  appId: 'dev',
);

const DeviceTarget kAndroidTarget = DeviceTarget(
  kind: TargetKind.android,
  addresses: <String>['10.0.0.5'],
  port: 7777,
);

const List<Device> kPlayOnDevices = <Device>[
  Device(
    id: 'd1',
    userId: 'u1',
    name: 'Living room',
    platform: 'roku',
    createdAt: '2026-10-14T18:00:00Z',
    target: kRokuTarget,
  ),
  Device(
    id: 'd2',
    userId: 'u1',
    name: 'Pixel',
    platform: 'android',
    createdAt: '2026-10-14T18:00:00Z',
    target: kAndroidTarget,
  ),
];

const List<PlayOnDevice> kPlayOnTargets = <PlayOnDevice>[
  PlayOnDevice(id: 'd1', name: 'Living room', target: kRokuTarget),
  PlayOnDevice(id: 'd2', name: 'Pixel', target: kAndroidTarget),
];

const PlayOnDevice kSharedRoku = PlayOnDevice(
  id: 's1',
  name: 'Den',
  owner: 'admin',
  target: kRokuTarget,
);

const String kRokuHome = '<active-app><app>Roku</app></active-app>';

typedef Mint = ({String device, String version});

class PlayOnServer {
  PlayOnServer({
    this.devices = kPlayOnDevices,
    this.shared = const <PlayOnDevice>[],
    this.devicesStatus = 200,
    this.mintStatus = 200,
    this.mintCode = 'version_not_found',
  });

  final List<Device> devices;
  final List<PlayOnDevice> shared;
  final int devicesStatus;
  final int mintStatus;
  final String mintCode;
  final List<Mint> mints = <Mint>[];
  final List<http.Request> lan = <http.Request>[];

  static final RegExp _commands = RegExp(r'^/api/v1/devices/([^/]+)/commands$');

  http.Response? answer(http.Request req) {
    if (req.url.path == '/api/v1/devices/shared') {
      return http.Response(
        jsonEncode(<dynamic>[for (final PlayOnDevice d in shared) d.toJson()]),
        200,
      );
    }
    if (req.url.path.endsWith('/devices')) {
      return devicesStatus == 200
          ? http.Response(
              jsonEncode(<dynamic>[for (final Device d in devices) d.toJson()]),
              200,
            )
          : http.Response('', devicesStatus);
    }
    final RegExpMatch? match = _commands.firstMatch(req.url.path);
    if (match == null) {
      return null;
    }
    final String device = match.group(1)!;
    final String version =
        (jsonDecode(req.body) as Map<String, dynamic>)['version_id'] as String;
    mints.add((device: device, version: version));
    if (mintStatus != 200) {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'error': <String, dynamic>{'code': mintCode, 'message': mintCode},
        }),
        mintStatus,
      );
    }
    final DeviceTarget target = <PlayOnDevice>[
      for (final Device d in devices) PlayOnDevice.own(d),
      ...shared,
    ].firstWhere((PlayOnDevice d) => d.id == device).target!;
    return http.Response(
      jsonEncode(<String, dynamic>{
        'token': 'tok-$version',
        'expires_at': '2026-10-14T18:03:11Z',
        'target': target.toJson(),
      }),
      200,
    );
  }

  PlayOnApi api(
    ApiClient client, {
    Future<http.Response> Function(http.Request req)? targets,
  }) => PlayOnApi(
    client,
    caller: TargetCaller(
      client: MockClient((http.Request req) {
        lan.add(req);
        return (targets ?? _answerTargets)(req);
      }),
    ),
  );

  static Future<http.Response> _answerTargets(http.Request req) async =>
      req.method == 'GET'
      ? http.Response(kRokuHome, 200)
      : http.Response('', req.url.path == '/play' ? 202 : 200);

  List<String> get lanCalls => <String>[
    for (final http.Request r in lan) '${r.method} ${r.url.host}${r.url.path}',
  ];
}
