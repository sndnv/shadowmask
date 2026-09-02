import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/util/job_labels.dart';

Map<String, dynamic> _job(String kind) => <String, dynamic>{
  'id': 'j1',
  'kind': kind,
  'status': 'succeeded',
  'priority': 'normal',
  'created_at': '2026-08-31T09:00:00Z',
  'updated_at': '2026-08-31T09:00:00Z',
};

void main() {
  test('every kind the server can send parses', () {
    // These three run on a schedule, so they only ever appear in the finished
    // history. Missing them made the jobs list fail on All but not on Active.
    const List<String> kinds = <String>[
      'library_scan',
      'metadata',
      'artwork',
      'subtitles',
      'trickplay',
      'fingerprint',
      'dedup',
      'cache_eviction',
      'search_reindex',
      'ingest',
      'relink',
      'transcription',
      'translation',
      'upscale',
      'combine',
      'fetch',
      'scheduled_scan',
      'retention',
      'orphan_sweep',
    ];

    expect(kinds.length, JobKind.values.length);
    for (final String kind in kinds) {
      expect(
        () => Job.fromJson(_job(kind)),
        returnsNormally,
        reason: '$kind must not throw while decoding a job',
      );
    }
  });

  test('every kind has a label', () {
    for (final JobKind kind in JobKind.values) {
      expect(jobKindLabel(kind), isNotEmpty);
    }
  });
}
