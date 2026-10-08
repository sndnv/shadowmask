import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/server/benchmark_report.dart';
import 'package:shadowmask/model/server/benchmark_run.dart';

void main() {
  test('a running benchmark reads its source, progress and runs', () {
    final BenchmarkReport r = BenchmarkReport.fromJson(<String, dynamic>{
      'state': 'running',
      'version_id': 'v1',
      'started_at': '2026-10-08T09:00:00Z',
      'finished_at': null,
      'source': <String, dynamic>{
        'codec': 'hevc',
        'width': 3840,
        'height': 2160,
        'hdr': 'hdr10',
        'duration_ms': 7200000,
      },
      'keyframes_ms': 1500,
      'keyframes_error': null,
      'progress': <String, dynamic>{'done': 2, 'total': 6},
      'runs': <dynamic>[
        <String, dynamic>{
          'segment': 450,
          'start_ms': 1800000,
          'duration_ms': 4000,
          'encoder': 'vaapi',
          'outcome': 'failed',
          'elapsed_ms': 300,
          'realtime': 13.33,
          'detail': 'No usable encoding entrypoint',
        },
        <String, dynamic>{
          'segment': 450,
          'start_ms': 1800000,
          'duration_ms': 4000,
          'encoder': 'software',
          'outcome': 'too_slow',
          'elapsed_ms': 60000,
          'realtime': 0,
          'detail': null,
        },
      ],
    });

    expect(r.state, BenchmarkState.running);
    expect(r.versionId, 'v1');
    expect(r.source?.codec, 'hevc');
    expect(r.source?.hdr, HdrFormat.hdr10);
    expect(r.source?.durationMs, 7200000);
    expect(r.keyframesMs, 1500);
    expect(r.progress.done, 2);
    expect(r.progress.total, 6);
    expect(r.runs.first.outcome, BenchmarkOutcome.failed);
    expect(r.runs.first.realtime, 13.33);
    expect(r.runs.last.outcome, BenchmarkOutcome.tooSlow);
    expect(r.runs.last.realtime, 0.0);
  });

  test('an idle benchmark has nothing yet', () {
    final BenchmarkReport r = BenchmarkReport.fromJson(<String, dynamic>{
      'state': 'idle',
    });

    expect(r.state, BenchmarkState.idle);
    expect(r.versionId, isNull);
    expect(r.progress.total, 0);
    expect(r.runs, isEmpty);
  });
}
