import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/util/job_labels.dart';

const String _start = '2026-08-17T07:05:00Z';

Job _job(JobStatus status, {String? started, double progress = 0.5}) => Job(
  id: 'j1',
  kind: JobKind.metadata,
  status: status,
  priority: JobPriority.normal,
  progress: progress,
  startedAt: started,
  createdAt: '2026-08-17T07:00:00Z',
  updatedAt: '2026-08-17T07:00:00Z',
);

void main() {
  test('a running job reads its start and its progress', () {
    expect(
      jobNarrowLine(_job(JobStatus.running, started: _start)),
      '${dateTimeText(_start)} · 50%',
    );
    expect(
      jobNarrowLine(_job(JobStatus.running, started: _start, progress: 0.126)),
      '${dateTimeText(_start)} · 13%',
    );
  });

  test('a job that has ended reads its start alone', () {
    for (final JobStatus status in <JobStatus>[
      JobStatus.succeeded,
      JobStatus.failed,
      JobStatus.cancelled,
    ]) {
      expect(
        jobNarrowLine(_job(status, started: _start)),
        dateTimeText(_start),
        reason: status.name,
      );
    }
  });

  test(
    'a queued job has no line, a running one without a start its progress',
    () {
      expect(jobNarrowLine(_job(JobStatus.queued)), isNull);
      expect(jobNarrowLine(_job(JobStatus.running)), '50%');
    },
  );
}
