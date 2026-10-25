import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/session/capability_overrides.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/player/diagnostics_report.dart';

void main() {
  test('a report holds the client, its capabilities, history, panel and '
      'log', () {
    const ClientDecoding measured = ClientDecoding(maxHeight: 2160);
    const ClientDecoding sent = ClientDecoding(maxHeight: 1080);
    const CapabilityOverrides choices = CapabilityOverrides(maxHeight: 1080);

    final Map<String, Object?> payload = diagnosticsPayload(
      platform: 'android 14',
      appVersion: '0.0.7',
      measured: measured,
      choices: choices,
      sent: sent,
      history: <Map<String, Object?>>[
        <String, Object?>{'at': '2026-10-24T09:00:00.000Z', 'event': 'opened'},
      ],
      panel: <String>['source: hevc 3840x2160 @ 24fps'],
      log: <String>['09:00:01 loadedmetadata at 0.0/5400.0 s, buffered none'],
    );

    expect(payload, <String, Object?>{
      'schema': 1,
      'client': <String, Object?>{
        'app': 'flutter',
        'platform': 'android 14',
        'version': '0.0.7',
      },
      'capabilities': <String, Object?>{
        'measured': measured.toJson(),
        'choices': choices.toJson(),
        'sent': sent.toJson(),
      },
      'history': <Map<String, Object?>>[
        <String, Object?>{'at': '2026-10-24T09:00:00.000Z', 'event': 'opened'},
      ],
      'panel': <String>['source: hevc 3840x2160 @ 24fps'],
      'log': <String>['09:00:01 loadedmetadata at 0.0/5400.0 s, buffered none'],
    });
    expect(
      () => jsonEncode(payload),
      returnsNormally,
      reason: 'it goes to the server as JSON',
    );
  });

  test('a client that measured nothing says so', () {
    final Map<String, Object?> payload = diagnosticsPayload(
      platform: 'linux',
      appVersion: '0.0.7',
      measured: null,
      choices: const CapabilityOverrides(),
      sent: null,
      history: const <Map<String, Object?>>[],
      panel: const <String>[],
      log: const <String>[],
    );

    expect(payload['capabilities'], <String, Object?>{
      'measured': null,
      'choices': const CapabilityOverrides().toJson(),
      'sent': null,
    });
  });
}
