import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/model/session/playback_session.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/player/actors/lifecycle.dart';
import 'package:shadowmask/player/actors/player.dart';
import 'package:shadowmask/player/actors/server.dart';
import 'package:shadowmask/player/actors/viewer.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_notice.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_step.dart';
import 'package:shadowmask/player/state/playback_stream.dart';
import 'package:shadowmask/player/state/playback_title.dart';
import 'package:shadowmask/player/state/player_tracks.dart';
import 'package:shadowmask/player/state/sent_change.dart';
import 'package:shadowmask/view/episode_neighbours.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/subtitle_choice.dart';

import '../../support/playback_run.dart';

const PlaybackControls k720 = PlaybackControls(
  height: 720,
  delivery: DeliveryPreference.auto,
);

const TrickplayRef kSheets = TrickplayRef(
  intervalMs: 10000,
  columns: 5,
  rows: 5,
  sheets: 2,
);

const EpisodeLink kPrevious = EpisodeLink(
  id: 'e1',
  seasonId: 'se1',
  label: 'E1',
);

const EpisodeLink kNext = EpisodeLink(id: 'e3', seasonId: 'se1', label: 'E3');

PlaybackRun _failedUpdate() {
  final PlaybackRun run = PlaybackRun.watching(session: kTranscode)
    ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
  run.apply((PlaybackState s) => serverFailed(s, run.sentId));
  return run;
}

PlaybackWrite _started(
  int id, {
  PlaybackSession session = kDirect,
  int positionMs = 0,
  bool seeksInStream = true,
}) =>
    (PlaybackState s) => serverStarted(
      s,
      id: id,
      session: session,
      positionMs: positionMs,
      seeksInStream: seeksInStream,
    );

void main() {
  test('a start answer fills the stream, takes the server track picks and '
      'loads where it starts', () {
    final PlaybackRun run = PlaybackRun.opened(resumeMs: 125000);
    final PlaybackSession session = kTranscode.copyWith(
      originMs: 120000,
      heartbeatIntervalS: 15,
      trickplay: const <TrickplayRef>[kSheets],
      selected: const SelectedTracks(
        audioTrack: 2,
        subtitleTrack: SubtitleSelection.embedded(4),
        subtitleDelivery: 'hls_vtt',
      ),
    );
    const PlaybackStream stream = PlaybackStream(
      sessionId: 's1',
      manifestUrl: '/transcode.m3u8',
      originMs: 120000,
      sequential: true,
      mode: PlaybackMode.transcode,
      container: 'fmp4',
      selected: SelectedTracks(
        audioTrack: 2,
        subtitleTrack: SubtitleSelection.embedded(4),
        subtitleDelivery: 'hls_vtt',
      ),
      heartbeatIntervalS: 15,
      seeksInStream: true,
      trickplay: <TrickplayRef>[kSheets],
    );

    expect(
      run.apply(_started(1, session: session, positionMs: 125000)),
      const <PlaybackCommand>[
        LoadStream(
          id: 1,
          stream: stream,
          offsetMs: 5000,
          tracks: PlayerTracks(rendition: true),
        ),
      ],
    );
    expect(run.state.stream, stream);
    expect(run.state.remote.audioTrack, 2);
    expect(
      run.state.remote.subtitle,
      const SubtitleChoice.on(SubtitleSelection.embedded(4)),
    );
    expect(
      run.state.status.sent,
      const SentChange.load(id: 1, positionMs: 125000),
    );
    expect(run.state.timeline.positionMs, 125000);
  });

  test('a stream the player cannot seek inside loads from its own start', () {
    final PlaybackRun run = PlaybackRun.opened(resumeMs: 125000);

    final LoadStream load =
        run
                .apply(
                  _started(
                    1,
                    session: kTranscode.copyWith(originMs: 120000),
                    positionMs: 125000,
                    seeksInStream: false,
                  ),
                )
                .single
            as LoadStream;

    expect(load.offsetMs, 0);
  });

  test('a start answer for a start no longer in flight ends its session', () {
    final PlaybackRun left = PlaybackRun.opened()..apply(lifecycleLeaves);
    expect(left.apply(_started(1)), const <PlaybackCommand>[
      EndSession(sessionId: 's1'),
    ]);

    final PlaybackRun away = PlaybackRun.opened()..apply(lifecycleBackgrounds);
    expect(away.apply(_started(1)), const <PlaybackCommand>[
      EndSession(sessionId: 's1'),
    ]);

    final PlaybackRun retried = PlaybackRun.opened();
    retried
      ..apply((PlaybackState s) => serverRefusedStart(s, 1, 'no'))
      ..apply(viewerRetries);
    final PlaybackState before = retried.state;
    expect(retried.apply(_started(1)), const <PlaybackCommand>[
      EndSession(sessionId: 's1'),
    ]);
    expect(retried.state, before);
  });

  test('a start holds for the buffer only when waiting for it and playing', () {
    const LocalSettings wait = LocalSettings(waitForBuffer: true);
    final PlaybackRun playing = PlaybackRun.opened(local: wait)
      ..apply(_started(1));
    expect(playing.state.status.holding, isTrue);
    expect(
      playing.apply((PlaybackState s) => playerLoaded(s, 1)),
      isEmpty,
      reason: 'it stays paused until the buffer is ready',
    );

    final PlaybackRun paused = PlaybackRun.opened(local: wait, playing: false)
      ..apply(_started(1));
    expect(paused.state.status.holding, isFalse);
  });

  test('a refused start shows its error and unblocks', () {
    final PlaybackRun run = PlaybackRun.opened();

    expect(
      run.apply((PlaybackState s) => serverRefusedStart(s, 1, 'no')),
      isEmpty,
    );
    expect(run.state.status.error, 'no');
    expect(run.state.status.sent, isNull);
  });

  test('an update answer replaces the stream and loads where it was asked', () {
    final PlaybackRun run = PlaybackRun.watching(atMs: 90000)
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    final int id = run.sentId;

    final LoadStream load =
        run
                .apply(
                  (PlaybackState s) => serverUpdated(
                    s,
                    id: id,
                    negotiation: kRenegotiated.copyWith(
                      originMs: 90000,
                      sequential: true,
                      selected: const SelectedTracks(
                        subtitleDelivery: 'burned',
                      ),
                    ),
                    seeksInStream: true,
                  ),
                )
                .single
            as LoadStream;

    expect(load.offsetMs, 0);
    expect(load.stream.manifestUrl, '/next.m3u8');
    expect(run.state.stream?.selected?.subtitleDelivery, 'burned');
    expect(run.state.remote, k720);
    expect(run.state.status.sent, SentChange.load(id: id, positionMs: 90000));
    expect(
      run.apply((PlaybackState s) => playerLoaded(s, id)),
      const <PlaybackCommand>[
        PlayPlayer(),
        SendProgress(sessionId: 's1', positionMs: 90000, playing: true),
      ],
    );
    expect(run.state.status.sent, isNull);
  });

  test('a seek answer brings the new stream\'s subtitle file', () {
    final PlaybackRun run = PlaybackRun.watching(
      session: kTranscode.copyWith(
        selected: const SelectedTracks(
          subtitleDelivery: 'external',
          subtitleUrl: '/stream/t1/subtitles.vtt',
        ),
      ),
    )..apply((PlaybackState s) => viewerSeeks(s, 600000));
    final int id = run.sentId;

    final LoadStream load =
        run
                .apply(
                  (PlaybackState s) => serverSeeked(
                    s,
                    id: id,
                    negotiation: kRenegotiated.copyWith(
                      originMs: 600000,
                      sequential: true,
                      selected: const SelectedTracks(
                        subtitleDelivery: 'external',
                        subtitleUrl: '/stream/t2/subtitles.vtt',
                      ),
                    ),
                    seeksInStream: true,
                  ),
                )
                .single
            as LoadStream;

    expect(load.offsetMs, 0);
    expect(
      load.tracks,
      const PlayerTracks(subtitleUrl: '/stream/t2/subtitles.vtt'),
    );
    expect(run.state.stream?.originMs, 600000);
    expect(run.state.timeline.positionMs, 600000);
  });

  test('a seek answer naming no tracks keeps those of the stream before', () {
    final PlaybackRun run = PlaybackRun.watching(
      session: kTranscode.copyWith(
        selected: const SelectedTracks(subtitleDelivery: 'hls_vtt'),
      ),
    )..apply((PlaybackState s) => viewerSeeks(s, 600000));
    final int id = run.sentId;

    final LoadStream load =
        run
                .apply(
                  (PlaybackState s) => serverSeeked(
                    s,
                    id: id,
                    negotiation: kRenegotiated.copyWith(originMs: 600000),
                    seeksInStream: true,
                  ),
                )
                .single
            as LoadStream;

    expect(run.state.stream?.selected?.subtitleDelivery, 'hls_vtt');
    expect(load.tracks, const PlayerTracks(rendition: true));
  });

  test('a failed update puts the settings back, says so and resumes', () {
    final PlaybackRun run = PlaybackRun.watching();
    final PlaybackControls before = run.state.remote;
    run.apply((PlaybackState s) => viewerChangesRemote(s, k720));

    expect(
      run.apply((PlaybackState s) => serverFailed(s, run.sentId)),
      const <PlaybackCommand>[
        PlayPlayer(),
        SendProgress(sessionId: 's1', positionMs: 0, playing: true),
      ],
    );
    expect(run.state.remote, before);
    expect(run.state.status.notice, PlaybackNotice.changeFailed);
    expect(run.state.status.sent, isNull);
  });

  test('a failed update leaves a paused video paused', () {
    final PlaybackRun run = PlaybackRun.watching(playing: false)
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));

    expect(
      run.apply((PlaybackState s) => serverFailed(s, run.sentId)),
      isEmpty,
    );
  });

  test('a failed seek puts the position back and says so', () {
    final PlaybackRun run = PlaybackRun.watching(session: kTranscode)
      ..apply((PlaybackState s) => viewerSeeks(s, 600000));

    expect(
      run.apply((PlaybackState s) => serverFailed(s, run.sentId)),
      const <PlaybackCommand>[
        PlayPlayer(),
        SendProgress(sessionId: 's1', positionMs: 0, playing: true),
      ],
    );
    expect(run.state.timeline.positionMs, 0);
    expect(run.state.status.notice, PlaybackNotice.seekFailed);
  });

  test('answers for another change are ignored', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    final PlaybackState before = run.state;

    for (final PlaybackWrite write in <PlaybackWrite>[
      (PlaybackState s) => serverUpdated(
        s,
        id: 1,
        negotiation: kRenegotiated,
        seeksInStream: true,
      ),
      (PlaybackState s) => serverSeeked(
        s,
        id: run.sentId,
        negotiation: kRenegotiated,
        seeksInStream: true,
      ),
      (PlaybackState s) => serverFailed(s, 1),
      (PlaybackState s) => serverRefusedStart(s, run.sentId, 'no'),
    ]) {
      expect(run.apply(write), isEmpty);
      expect(run.state, before);
    }
  });

  test('the next change clears the notice of the one that failed', () {
    for (final List<PlaybackWrite> next in <List<PlaybackWrite>>[
      <PlaybackWrite>[(PlaybackState s) => viewerChangesRemote(s, k720)],
      <PlaybackWrite>[(PlaybackState s) => viewerSeeks(s, 600000)],
      <PlaybackWrite>[
        (PlaybackState s) => playerErrs(s, 'boom'),
        viewerRetries,
      ],
      <PlaybackWrite>[
        lifecycleBackgrounds,
        (PlaybackState s) => lifecycleForegrounds(s, playing: true),
      ],
    ]) {
      final PlaybackRun run = _failedUpdate();
      expect(run.state.status.notice, PlaybackNotice.changeFailed);

      next.forEach(run.apply);

      expect(run.state.status.sent, isNotNull);
      expect(run.state.status.notice, isNull);
    }
  });

  test('the title takes its name and neighbours', () {
    final PlaybackRun run = PlaybackRun.watching();

    expect(
      run.apply(
        (PlaybackState s) => serverDescribesTitle(
          s,
          name: 'The Show · S01E02',
          previous: kPrevious,
          next: kNext,
        ),
      ),
      isEmpty,
    );

    expect(
      run.state.title,
      const PlaybackTitle(
        version: kVersion,
        name: 'The Show · S01E02',
        previous: kPrevious,
        next: kNext,
      ),
    );
  });

  test('a fresh version replaces the old one and keeps a known length', () {
    final PlaybackRun run = PlaybackRun.watching();
    final VersionDetail fresh = kVersion.copyWith(container: 'mp4');

    expect(
      run.apply((PlaybackState s) => serverRefreshesVersion(s, fresh)),
      isEmpty,
    );
    expect(run.state.title.version, fresh);

    run.apply(
      (PlaybackState s) =>
          serverRefreshesVersion(s, fresh.copyWith(durationMs: 0)),
    );
    expect(run.state.title.version, fresh);

    final (PlaybackState unknown, _) = serverRefreshesVersion(
      const PlaybackState(versionId: 'v1'),
      fresh.copyWith(durationMs: 0),
    );
    expect(unknown.title.version?.durationMs, 0);
  });
}
