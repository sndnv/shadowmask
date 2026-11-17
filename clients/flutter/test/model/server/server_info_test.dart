import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/server/server_info.dart';

void main() {
  test('ServerInfo.fromJson defaults profile_version to 1', () {
    expect(ServerInfo.fromJson(<String, dynamic>{}).profileVersion, 1);
    expect(
      ServerInfo.fromJson(<String, dynamic>{
        'profile_version': 3,
      }).profileVersion,
      3,
    );
  });

  test('ServerInfo.fromJson reads the uptime an admin is given', () {
    final ServerInfo info = ServerInfo.fromJson(<String, dynamic>{
      'started_at': '2026-11-17T09:53:40Z',
      'uptime_ms': 190800000,
    });

    expect(info.startedAt, '2026-11-17T09:53:40Z');
    expect(info.uptimeMs, 190800000);
  });

  test('ServerInfo.fromJson has no uptime without the fields', () {
    final ServerInfo info = ServerInfo.fromJson(<String, dynamic>{});

    expect(info.startedAt, isNull);
    expect(info.uptimeMs, isNull);
  });
}
