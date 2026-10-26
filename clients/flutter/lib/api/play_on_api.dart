import 'package:flutter/foundation.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/target_caller.dart';
import 'package:shadowmask/model/auth/device.dart';
import 'package:shadowmask/model/auth/play_command.dart';
import 'package:shadowmask/model/auth/play_on_device.dart';
import 'package:shadowmask/model/catalog/version.dart';

PlayOnApi? playOnApi(ApiClient api) => kIsWeb ? null : PlayOnApi(api);

final TargetCaller _sharedCaller = TargetCaller();

class PlayOnApi {
  PlayOnApi(this._api, {TargetCaller? caller}) : _lan = caller;

  final ApiClient _api;
  final TargetCaller? _lan;

  Future<List<PlayOnDevice>> targets(String userId) async {
    final List<Device> devices = await _api.getJsonArray(
      '/api/v1/users/${_enc(userId)}/devices',
      Device.fromJson,
    );
    final List<PlayOnDevice> shared = await _api
        .getJsonArray('/api/v1/devices/shared', PlayOnDevice.fromJson)
        .catchError((Object _) => <PlayOnDevice>[]);
    final String self = await _api.linkedDeviceId();
    return <PlayOnDevice>[
      for (final Device d in devices)
        if (d.target?.kind != null && d.id != self) PlayOnDevice.own(d),
      for (final PlayOnDevice d in shared)
        if (d.target?.kind != null) d,
    ];
  }

  Future<PlayCommand> mint(String deviceId, String versionId) => _api.postJson(
    '/api/v1/devices/${_enc(deviceId)}/commands',
    <String, dynamic>{'version_id': versionId},
    PlayCommand.fromJson,
  );

  Future<PlayOnOutcome> send(PlayOnDevice device, Version version) async =>
      (_lan ?? _sharedCaller).send(await mint(device.id, version.id), version);

  Future<void> reportTarget(List<String> addresses, int port) => _api.sendVoid(
    'PUT',
    '/api/v1/devices/self/target',
    body: <String, dynamic>{
      'kind': 'android',
      'addresses': addresses,
      'port': port,
    },
  );

  Future<void> clearTarget() =>
      _api.sendVoid('DELETE', '/api/v1/devices/self/target');

  Future<String> redeem(String token) => _api.postJson(
    '/api/v1/devices/self/commands/redeem',
    <String, String>{'token': token},
    (Map<String, dynamic> json) => json['version_id'] as String,
  );

  String _enc(String s) => Uri.encodeComponent(s);
}
