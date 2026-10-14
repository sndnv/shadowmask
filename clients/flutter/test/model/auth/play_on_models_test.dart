import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/auth/device.dart';
import 'package:shadowmask/model/auth/device_target.dart';
import 'package:shadowmask/model/auth/play_command.dart';

Map<String, dynamic> _device(Object? target) => <String, dynamic>{
  'id': 'd1',
  'user_id': 'u1',
  'name': 'Living room',
  'platform': 'roku',
  'created_at': '2026-10-14T18:00:00Z',
  'target': target,
};

void main() {
  test('a device carries its target report', () {
    final Device d = Device.fromJson(
      _device(<String, dynamic>{
        'kind': 'roku',
        'addresses': <String>['192.168.1.40'],
        'port': 8060,
        'app_id': 'dev',
        'reported_at': '2026-10-14T18:02:11Z',
      }),
    );

    expect(
      d.target,
      const DeviceTarget(
        kind: TargetKind.roku,
        addresses: <String>['192.168.1.40'],
        port: 8060,
        appId: 'dev',
        reportedAt: '2026-10-14T18:02:11Z',
      ),
    );
  });

  test('a device that never reported has no target', () {
    expect(Device.fromJson(_device(null)).target, isNull);
  });

  test('a kind this app does not know still reads, without a kind', () {
    final Device d = Device.fromJson(
      _device(<String, dynamic>{
        'kind': 'television',
        'addresses': <String>['192.168.1.40'],
        'port': 8060,
      }),
    );

    expect(d.name, 'Living room');
    expect(d.target?.kind, isNull);
    expect(d.target?.addresses, <String>['192.168.1.40']);
  });

  test('an android target has no app id', () {
    final DeviceTarget t = DeviceTarget.fromJson(<String, dynamic>{
      'kind': 'android',
      'addresses': <String>['10.0.0.5', '10.0.0.6'],
      'port': 7777,
    });

    expect(t.kind, TargetKind.android);
    expect(t.addresses, <String>['10.0.0.5', '10.0.0.6']);
    expect(t.appId, isNull);
    expect(t.reportedAt, isNull);
  });

  test('a play command carries its token and where to send it', () {
    final PlayCommand c = PlayCommand.fromJson(<String, dynamic>{
      'token': 'eyJ.x.y',
      'expires_at': '2026-10-14T18:03:11Z',
      'target': <String, dynamic>{
        'kind': 'roku',
        'addresses': <String>['192.168.1.40'],
        'port': 8060,
        'app_id': 'dev',
      },
    });

    expect(c.token, 'eyJ.x.y');
    expect(c.expiresAt, '2026-10-14T18:03:11Z');
    expect(c.target.appId, 'dev');
  });
}
