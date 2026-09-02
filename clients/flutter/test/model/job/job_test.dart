import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/job/job.dart';

void main() {
  test('Job parses enums, parent and cancellable', () {
    final Job j = Job.fromJson(<String, dynamic>{
      'id': 'j1',
      'kind': 'library_scan',
      'status': 'running',
      'priority': 'high',
      'progress': 0.5,
      'attempts': 2,
      'last_error': 'boom',
      'created_at': '2026-01-01T00:00:00Z',
      'updated_at': '2026-01-02T00:00:00Z',
      'started_at': '2026-01-01T00:05:00Z',
      'parent_id': 'p1',
      'cancellable': true,
    });

    expect(j.kind, JobKind.libraryScan);
    expect(j.status, JobStatus.running);
    expect(j.priority, JobPriority.high);
    expect(j.progress, 0.5);
    expect(j.attempts, 2);
    expect(j.lastError, 'boom');
    expect(j.parentId, 'p1');
    expect(j.cancellable, isTrue);
  });

  test('Job applies defaults for missing optional fields', () {
    final Job j = Job.fromJson(<String, dynamic>{
      'id': 'j2',
      'kind': 'fetch',
      'status': 'queued',
      'priority': 'normal',
      'created_at': '2026-01-01T00:00:00Z',
      'updated_at': '2026-01-01T00:00:00Z',
    });

    expect(j.kind, JobKind.fetch);
    expect(j.progress, 0);
    expect(j.attempts, 0);
    expect(j.cancellable, isFalse);
    expect(j.parentId, isNull);
    expect(j.lastError, isNull);
  });
}
