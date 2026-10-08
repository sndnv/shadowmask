import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/server/capabilities_report.dart';
import 'package:shadowmask/model/server/hardware_test.dart';

void main() {
  test('a ready report reads every part', () {
    final CapabilitiesReport r = CapabilitiesReport.fromJson(<String, dynamic>{
      'state': 'ready',
      'checked_at': '2026-10-08T09:00:00Z',
      'host': <String, dynamic>{
        'cpu': '12th Gen Intel(R) Core(TM) i3-1220P',
        'cores': 10,
        'threads': 12,
      },
      'ffmpeg': <String, dynamic>{
        'version': '5.1.6-0+deb12u1',
        'error': null,
        'hwaccels': <String>['vdpau', 'vaapi'],
        'encoders': <dynamic>[
          <String, dynamic>{'name': 'libx264', 'present': true},
          <String, dynamic>{'name': 'h264_vaapi', 'present': false},
        ],
        'filters': <dynamic>[
          <String, dynamic>{'name': 'zscale', 'present': true},
        ],
      },
      'hardware': <String, dynamic>{
        'mode': 'auto',
        'device': '/dev/dri/renderD128',
        'device_present': true,
        'in_use': true,
        'test': <String, dynamic>{
          'outcome': 'works',
          'low_power': true,
          'elapsed_ms': 420,
          'detail': null,
          'low_power_detail': null,
        },
      },
    });

    expect(r.state, CheckState.ready);
    expect(r.checkedAt, '2026-10-08T09:00:00Z');
    expect(r.host?.cpu, '12th Gen Intel(R) Core(TM) i3-1220P');
    expect(r.host?.cores, 10);
    expect(r.host?.threads, 12);
    expect(r.ffmpeg?.version, '5.1.6-0+deb12u1');
    expect(r.ffmpeg?.hwaccels, <String>['vdpau', 'vaapi']);
    expect(r.ffmpeg?.encoders.last.name, 'h264_vaapi');
    expect(r.ffmpeg?.encoders.last.present, isFalse);
    expect(r.ffmpeg?.filters.single.present, isTrue);
    expect(r.hardware?.devicePresent, isTrue);
    expect(r.hardware?.inUse, isTrue);
    expect(r.hardware?.test.outcome, HardwareTestOutcome.works);
    expect(r.hardware?.test.lowPower, isTrue);
    expect(r.hardware?.test.elapsedMs, 420);
  });

  test('an unchecked report has no parts', () {
    final CapabilitiesReport r = CapabilitiesReport.fromJson(<String, dynamic>{
      'state': 'unchecked',
      'checked_at': null,
      'host': null,
      'ffmpeg': null,
      'hardware': null,
    });

    expect(r.state, CheckState.unchecked);
    expect(r.host, isNull);
    expect(r.ffmpeg, isNull);
    expect(r.hardware, isNull);
  });

  test('a failed test keeps both errors', () {
    final HardwareTest test = HardwareTest.fromJson(<String, dynamic>{
      'outcome': 'failed',
      'low_power': null,
      'elapsed_ms': null,
      'detail': 'No usable encoding entrypoint',
      'low_power_detail': 'Function not implemented',
    });

    expect(test.outcome, HardwareTestOutcome.failed);
    expect(test.detail, 'No usable encoding entrypoint');
    expect(test.lowPowerDetail, 'Function not implemented');
  });
}
