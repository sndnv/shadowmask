import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/model/job/version_job.dart';

void main() {
  test('a version job reads every field the server sends', () {
    final VersionJob j = VersionJob.fromJson(<String, dynamic>{
      'id': 'j1',
      'kind': 'transcription',
      'status': 'queued',
      'ahead': 3,
      'created_at': '2026-10-06T10:00:00Z',
      'started_at': null,
      'finished_at': null,
      'elapsed_ms': null,
      'last_error': null,
      'language': 'en',
      'subtitle_id': 'generated:v1',
    });

    expect(j.kind, JobKind.transcription);
    expect(j.status, JobStatus.queued);
    expect(j.ahead, 3);
    expect(j.language, 'en');
    expect(j.subtitleId, 'generated:v1');
    expect(j.active, isTrue);
  });

  test('only queued and running work is active', () {
    VersionJob of(String status) => VersionJob.fromJson(<String, dynamic>{
      'id': 'j',
      'kind': 'translation',
      'status': status,
      'created_at': '2026-10-06T10:00:00Z',
    });

    expect(of('queued').active, isTrue);
    expect(of('running').active, isTrue);
    expect(of('succeeded').active, isFalse);
    expect(of('failed').active, isFalse);
    expect(of('cancelled').active, isFalse);
  });
}
