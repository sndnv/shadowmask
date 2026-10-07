import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/job/version_job.dart';
import 'package:shadowmask/view/version_job_status.dart';

VersionJob _job(String kind, String status, {Map<String, dynamic>? extra}) =>
    VersionJob.fromJson(<String, dynamic>{
      'id': 'j1',
      'kind': kind,
      'status': status,
      'created_at': '2026-10-06T10:00:00Z',
      ...?extra,
    });

VersionDetail _detail(List<Map<String, dynamic>> files) =>
    VersionDetail.fromJson(<String, dynamic>{
      'id': 'v1',
      'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      'library_id': 'l',
      'quality': 'fhd',
      'container': 'mkv',
      'subtitle_files': files,
    });

void main() {
  test('queued work says how much is ahead of it', () {
    String ahead(int? n) => versionJobStatus(
      _job('transcription', 'queued', extra: <String, dynamic>{'ahead': n}),
      null,
    );

    expect(ahead(0), 'Queued · next in line');
    expect(ahead(1), 'Queued · 1 job ahead');
    expect(ahead(4), 'Queued · 4 jobs ahead');
    expect(ahead(null), 'Queued · feature switched off');
  });

  test('running work says how long it has run', () {
    String running(int? ms) => versionJobStatus(
      _job(
        'translation',
        'running',
        extra: <String, dynamic>{'elapsed_ms': ms},
      ),
      null,
    );

    expect(running(760000), 'Running · 12m 40s');
    expect(running(9000), 'Running · 9s');
    expect(running(null), 'Running · 0s');
  });

  test('finished work is ready only when its subtitle is on the version', () {
    final VersionJob done = _job(
      'transcription',
      'succeeded',
      extra: <String, dynamic>{'language': 'en', 'subtitle_id': 'generated:v1'},
    );
    final VersionDetail held = _detail(<Map<String, dynamic>>[
      <String, dynamic>{
        'id': 'generated:v1',
        'format': 'srt',
        'source': 'generated',
      },
    ]);

    expect(versionJobStatus(done, held), 'Ready · English (generated) added');
    expect(
      versionJobStatus(done, _detail(<Map<String, dynamic>>[])),
      Strings.jobNoSubtitle,
    );
    expect(versionJobStatus(done, null), Strings.statusSucceeded);
  });

  test('a combined subtitle without a language still reads as ready', () {
    final VersionJob done = _job(
      'combine',
      'succeeded',
      extra: <String, dynamic>{'subtitle_id': 'combined:v1:a:b'},
    );
    final VersionDetail held = _detail(<Map<String, dynamic>>[
      <String, dynamic>{
        'id': 'combined:v1:a:b',
        'format': 'ass',
        'source': 'combined',
      },
    ]);

    expect(versionJobStatus(done, held), 'Ready · combined subtitle added');
  });

  test('finished work that names no subtitle reads as succeeded', () {
    expect(
      versionJobStatus(
        _job('translation', 'succeeded'),
        _detail(<Map<String, dynamic>>[]),
      ),
      Strings.statusSucceeded,
    );
  });

  test('failed and cancelled work say so', () {
    expect(
      versionJobStatus(
        _job(
          'translation',
          'failed',
          extra: <String, dynamic>{'last_error': 'provider refused'},
        ),
        null,
      ),
      'Failed · provider refused',
    );
    expect(
      versionJobStatus(_job('translation', 'failed'), null),
      Strings.statusFailed,
    );
    expect(
      versionJobStatus(_job('combine', 'cancelled'), null),
      Strings.statusCancelled,
    );
  });

  test('a failure gives its reason without the queue\'s prefix', () {
    String failed(String error) => versionJobStatus(
      _job(
        'transcription',
        'failed',
        extra: <String, dynamic>{'last_error': error},
      ),
      null,
    );

    expect(
      failed('permanent job failure: whisper exited with status 1'),
      'Failed · whisper exited with status 1',
    );
    expect(
      failed('Retryable job failure: provider refused'),
      'Failed · provider refused',
    );
    expect(failed('permanent job failure: '), Strings.statusFailed);
  });

  test('the title names the kind and the language asked for', () {
    expect(
      versionJobTitle(
        _job(
          'translation',
          'queued',
          extra: <String, dynamic>{'language': 'es'},
        ),
      ),
      'Translation · Spanish',
    );
    expect(versionJobTitle(_job('combine', 'queued')), 'Combine');
  });

  test('only subtitle work is shown to the viewer', () {
    final List<VersionJob> all = <VersionJob>[
      _job('transcription', 'queued'),
      _job('translation', 'queued'),
      _job('combine', 'queued'),
      _job('trickplay', 'queued'),
      _job('upscale', 'queued'),
      _job('subtitles', 'queued'),
      _job('relink', 'queued'),
    ];

    expect(subtitleWork(all).map((VersionJob j) => j.kind.name), <String>[
      'transcription',
      'translation',
      'combine',
    ]);
  });
}
