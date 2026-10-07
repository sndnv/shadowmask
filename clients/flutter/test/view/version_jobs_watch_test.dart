import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/model/job/version_job.dart';
import 'package:shadowmask/view/version_jobs_watch.dart';

VersionJob _job(
  String id,
  String status, {
  String kind = 'transcription',
  int? ahead = 0,
}) => VersionJob.fromJson(<String, dynamic>{
  'id': id,
  'kind': kind,
  'status': status,
  'ahead': ahead,
  'created_at': '2026-10-06T10:00:00Z',
});

const Duration _aWhile = Duration(minutes: 5);

Future<bool> _reread() async => true;

class _Server {
  _Server(this.answers);

  final List<Object> answers;
  int calls = 0;

  Future<List<VersionJob>> fetch() async {
    final Object answer =
        answers[calls < answers.length ? calls : answers.length - 1];
    calls++;
    if (answer is Exception) {
      throw answer;
    }
    return answer as List<VersionJob>;
  }
}

void main() {
  testWidgets('nothing is read until asked', (WidgetTester tester) async {
    final _Server server = _Server(<Object>[<VersionJob>[]]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: _reread,
    );
    await tester.pump(_aWhile);

    expect(server.calls, 0);
    watch.dispose();
  });

  testWidgets('with nothing running one read is all there is', (
    WidgetTester tester,
  ) async {
    final _Server server = _Server(<Object>[
      <VersionJob>[_job('j1', 'succeeded')],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: _reread,
    );
    watch.refresh();
    await tester.pump(_aWhile);

    expect(server.calls, 1);
    expect(watch.value, hasLength(1));
    watch.dispose();
  });

  testWidgets('the first read never counts as finished work', (
    WidgetTester tester,
  ) async {
    int finished = 0;
    final _Server server = _Server(<Object>[
      <VersionJob>[_job('j1', 'succeeded')],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: () async {
        finished++;
        return true;
      },
    );
    watch.refresh();
    await tester.pump();

    expect(finished, 0);
    watch.dispose();
  });

  testWidgets('open work is polled until it ends, then polling stops', (
    WidgetTester tester,
  ) async {
    int finished = 0;
    final _Server server = _Server(<Object>[
      <VersionJob>[_job('j1', 'queued')],
      <VersionJob>[_job('j1', 'running')],
      <VersionJob>[_job('j1', 'succeeded')],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: () async {
        finished++;
        return true;
      },
    );
    watch.refresh();
    await tester.pump();
    await tester.pump(kJobsBusyPoll);
    expect(server.calls, 2);
    expect(finished, 0);

    await tester.pump(kJobsBusyPoll);
    expect(server.calls, 3);
    expect(finished, 1);
    expect(watch.value.single.status, JobStatus.succeeded);

    await tester.pump(_aWhile);
    expect(server.calls, 3);
    expect(finished, 1);
    watch.dispose();
  });

  testWidgets('finished work shows only once the version was read again', (
    WidgetTester tester,
  ) async {
    final Completer<bool> reread = Completer<bool>();
    final _Server server = _Server(<Object>[
      <VersionJob>[_job('j1', 'running')],
      <VersionJob>[_job('j1', 'succeeded')],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: () => reread.future,
    );
    watch.refresh();
    await tester.pump();
    watch.refresh();
    await tester.pump();

    expect(watch.value.single.status, JobStatus.running);

    reread.complete(true);
    await tester.pump();
    expect(watch.value.single.status, JobStatus.succeeded);
    watch.dispose();
  });

  testWidgets('a failed read of the version keeps the work open and retries', (
    WidgetTester tester,
  ) async {
    final List<bool> answers = <bool>[false, true];
    int rereads = 0;
    final _Server server = _Server(<Object>[
      <VersionJob>[_job('j1', 'running')],
      <VersionJob>[_job('j1', 'succeeded')],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: () async => answers[rereads++],
    );
    watch.refresh();
    await tester.pump();
    await tester.pump(kJobsBusyPoll);

    expect(rereads, 1);
    expect(watch.value.single.status, JobStatus.running);

    await tester.pump(kJobsBusyPoll);
    expect(rereads, 2);
    expect(watch.value.single.status, JobStatus.succeeded);

    await tester.pump(_aWhile);
    expect(server.calls, 3);
    watch.dispose();
  });

  testWidgets('a throwing read of the version counts as failed', (
    WidgetTester tester,
  ) async {
    final _Server server = _Server(<Object>[
      <VersionJob>[_job('j1', 'running')],
      <VersionJob>[_job('j1', 'succeeded')],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: () async => throw Exception('offline'),
    );
    watch.refresh();
    await tester.pump();
    await tester.pump(kJobsBusyPoll);

    expect(watch.value.single.status, JobStatus.running);
    await tester.pump(kJobsBusyPoll);
    expect(server.calls, 3);
    watch.dispose();
  });

  testWidgets('work that ended between two reads is reported', (
    WidgetTester tester,
  ) async {
    int finished = 0;
    final _Server server = _Server(<Object>[
      <VersionJob>[],
      <VersionJob>[_job('j2', 'failed')],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: () async {
        finished++;
        return true;
      },
    );
    watch.refresh();
    await tester.pump();
    watch.refresh();
    await tester.pump();

    expect(finished, 1);
    watch.dispose();
  });

  testWidgets('only subtitle work is polled or reported', (
    WidgetTester tester,
  ) async {
    int finished = 0;
    final _Server server = _Server(<Object>[
      <VersionJob>[
        _job('u1', 'running', kind: 'upscale'),
        _job('t1', 'queued', kind: 'trickplay'),
      ],
      <VersionJob>[
        _job('u1', 'succeeded', kind: 'upscale'),
        _job('t1', 'succeeded', kind: 'trickplay'),
      ],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: () async {
        finished++;
        return true;
      },
    );
    watch.refresh();
    await tester.pump(_aWhile);
    expect(server.calls, 1);

    watch.refresh();
    await tester.pump();
    expect(finished, 0);
    watch.dispose();
  });

  testWidgets('work queued behind a switched-off feature is not polled', (
    WidgetTester tester,
  ) async {
    final _Server server = _Server(<Object>[
      <VersionJob>[_job('j1', 'queued', ahead: null)],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: _reread,
    );
    watch.refresh();
    await tester.pump(_aWhile);

    expect(server.calls, 1);
    expect(watch.value, hasLength(1));
    watch.dispose();
  });

  testWidgets('a failed read keeps what was shown and retries open work', (
    WidgetTester tester,
  ) async {
    final _Server server = _Server(<Object>[
      <VersionJob>[_job('j1', 'running')],
      Exception('offline'),
      <VersionJob>[_job('j1', 'running')],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: _reread,
    );
    watch.refresh();
    await tester.pump();
    await tester.pump(kJobsBusyPoll);

    expect(server.calls, 2);
    expect(watch.value, hasLength(1));

    await tester.pump(kJobsBusyPoll);
    expect(server.calls, 3);
    watch.dispose();
  });

  testWidgets('a failed read with nothing open does not retry', (
    WidgetTester tester,
  ) async {
    final _Server server = _Server(<Object>[Exception('offline')]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: _reread,
    );
    watch.refresh();
    await tester.pump(_aWhile);

    expect(server.calls, 1);
    watch.dispose();
  });

  testWidgets('a refresh supersedes the read in flight', (
    WidgetTester tester,
  ) async {
    final Completer<List<VersionJob>> slow = Completer<List<VersionJob>>();
    int calls = 0;
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: () {
        calls++;
        return calls == 1
            ? slow.future
            : Future<List<VersionJob>>.value(<VersionJob>[
                _job('j1', 'queued'),
              ]);
      },
      onFinished: _reread,
    );
    watch.refresh();
    watch.refresh();
    await tester.pump();
    expect(watch.value.single.id, 'j1');

    slow.complete(<VersionJob>[]);
    await tester.pump();
    expect(watch.value, hasLength(1));

    await tester.pump(kJobsBusyPoll);
    expect(calls, 3);
    watch.dispose();
  });

  testWidgets('a disposed watch stops polling', (WidgetTester tester) async {
    final _Server server = _Server(<Object>[
      <VersionJob>[_job('j1', 'running')],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: _reread,
    );
    watch.refresh();
    await tester.pump();
    watch.dispose();

    await tester.pump(_aWhile);
    expect(server.calls, 1);
  });

  testWidgets('a read landing after dispose is dropped', (
    WidgetTester tester,
  ) async {
    final Completer<List<VersionJob>> late = Completer<List<VersionJob>>();
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: () => late.future,
      onFinished: _reread,
    );
    watch.refresh();
    watch.dispose();

    late.complete(<VersionJob>[_job('j1', 'running')]);
    await tester.pump(_aWhile);
  });

  testWidgets('a version read landing after dispose is dropped', (
    WidgetTester tester,
  ) async {
    final Completer<bool> reread = Completer<bool>();
    final _Server server = _Server(<Object>[
      <VersionJob>[_job('j1', 'running')],
      <VersionJob>[_job('j1', 'succeeded')],
    ]);
    final VersionJobsWatch watch = VersionJobsWatch(
      fetch: server.fetch,
      onFinished: () => reread.future,
    );
    watch.refresh();
    await tester.pump();
    watch.refresh();
    await tester.pump();
    watch.dispose();

    reread.complete(true);
    await tester.pump(_aWhile);
    expect(server.calls, 2);
  });
}
