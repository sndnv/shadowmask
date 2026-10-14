import 'dart:async';
import 'dart:convert';

import 'package:http/http.dart' as http;

import 'package:shadowmask/model/auth/device_target.dart';
import 'package:shadowmask/model/auth/play_command.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/util/lan_client.dart';

enum PlayOnOutcome { playing, refused, unreachable }

const Duration kLanConnectTimeout = Duration(seconds: 2);
const Duration kEcpTimeout = Duration(seconds: 2);
const Duration kPlayTimeout = Duration(seconds: 10);

final RegExp _activeApp = RegExp(r'<app\b[^>]*\bid="([^"]*)"');

String? runningAppId(String body) => _activeApp.firstMatch(body)?.group(1);

class TargetCaller {
  TargetCaller({
    http.Client? client,
    this.ecpTimeout = kEcpTimeout,
    this.playTimeout = kPlayTimeout,
  }) : _http = client ?? lanClient(kLanConnectTimeout);

  static const Set<int> _refused = <int>{401, 403};

  final http.Client _http;
  final Duration ecpTimeout;
  final Duration playTimeout;

  Future<PlayOnOutcome> send(PlayCommand command, Version version) async {
    final DeviceTarget target = command.target;
    for (final String address in target.addresses) {
      final Uri base = Uri(scheme: 'http', host: address, port: target.port);
      final PlayOnOutcome? outcome = switch (target.kind) {
        TargetKind.roku => await _roku(base, target, command, version),
        TargetKind.android => await _android(base, command),
        null => null,
      };
      if (outcome != null) {
        return outcome;
      }
    }
    return PlayOnOutcome.unreachable;
  }

  Future<PlayOnOutcome?> _roku(
    Uri base,
    DeviceTarget target,
    PlayCommand command,
    Version version,
  ) async {
    final String? appId = target.appId;
    if (appId == null) {
      return null;
    }
    final http.Response? active = await _request(
      () => _http.get(base.replace(path: '/query/active-app')),
      ecpTimeout,
    );
    if (active == null || _refused.contains(active.statusCode)) {
      return _outcome(active);
    }
    final bool running = runningAppId(active.body) == appId;
    final Uri uri = base.replace(
      pathSegments: running ? <String>['input'] : <String>['launch', appId],
      queryParameters: <String, String>{
        'contentId': version.id,
        'mediaType': version.title.type.wire,
        'command': command.token,
      },
    );
    return _outcome(await _request(() => _http.post(uri), ecpTimeout));
  }

  Future<PlayOnOutcome?> _android(Uri base, PlayCommand command) async =>
      _outcome(
        await _request(
          () => _http.post(
            base.replace(path: '/play'),
            headers: <String, String>{'Content-Type': 'application/json'},
            body: jsonEncode(<String, String>{'token': command.token}),
          ),
          playTimeout,
        ),
      );

  Future<http.Response?> _request(
    Future<http.Response> Function() send,
    Duration timeout,
  ) async {
    try {
      return await send().timeout(timeout);
    } catch (_) {
      return null;
    }
  }

  PlayOnOutcome? _outcome(http.Response? res) {
    if (res == null) {
      return null;
    }
    if (_refused.contains(res.statusCode)) {
      return PlayOnOutcome.refused;
    }
    if (res.statusCode >= 200 && res.statusCode < 300) {
      return PlayOnOutcome.playing;
    }
    return null;
  }
}
