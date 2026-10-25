import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/model/session/negotiation.dart';
import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/model/session/playback_session.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/player/playback_history.dart';

void main() {
  test('an entry carries its time in UTC, its event and its facts', () {
    final PlaybackHistory history = PlaybackHistory(
      now: () => DateTime.utc(2026, 10, 24, 9, 0, 1, 120),
    );

    history.add('opened');
    history.add('sent_seek', <String, Object?>{
      'session': 's1',
      'position_ms': 5000,
    });

    expect(history.entries, <Map<String, Object?>>[
      <String, Object?>{'at': '2026-10-24T09:00:01.120Z', 'event': 'opened'},
      <String, Object?>{
        'at': '2026-10-24T09:00:01.120Z',
        'event': 'sent_seek',
        'session': 's1',
        'position_ms': 5000,
      },
    ]);
  });

  test('it keeps the newest entries up to its limit', () {
    final PlaybackHistory history = PlaybackHistory();

    for (int n = 0; n < kPlaybackHistoryLimit + 3; n++) {
      history.add('e$n');
    }

    expect(history.entries.length, kPlaybackHistoryLimit);
    expect(history.entries.first['event'], 'e3');
    expect(history.entries.last['event'], 'e${kPlaybackHistoryLimit + 2}');
  });

  test('an answer is told by its technical facts, never its URLs', () {
    const SelectedTracks selected = SelectedTracks(
      audioTrack: 2,
      audioDelivery: 'muxed',
      subtitleTrack: SubtitleSelection.embedded(3),
      subtitleDelivery: 'external',
      subtitleUrl: '/stream/secret-token/subtitles.vtt',
    );
    const PlaybackSession session = PlaybackSession(
      sessionId: 's1',
      mode: PlaybackMode.remux,
      container: 'fmp4',
      manifestUrl: '/stream/secret-token/master.m3u8',
      originMs: 4000,
      sequential: true,
      selected: selected,
    );
    const Map<String, Object?> tracks = <String, Object?>{
      'audio_track': 2,
      'audio_delivery': 'muxed',
      'subtitle_track': <String, dynamic>{'type': 'embedded', 'index': 3},
      'subtitle_delivery': 'external',
    };

    expect(startedFacts(session), <String, Object?>{
      'request': 'start',
      'session': 's1',
      'mode': 'remux',
      'container': 'fmp4',
      'origin_ms': 4000,
      'sequential': true,
      'selected': tracks,
    });
    const Negotiation negotiation = Negotiation(
      manifestUrl: '/stream/secret-token/master.m3u8',
      mode: PlaybackMode.transcode,
      selected: selected,
    );
    expect(negotiatedFacts('seek', 's1', negotiation), <String, Object?>{
      'request': 'seek',
      'session': 's1',
      'mode': 'transcode',
      'container': null,
      'origin_ms': 0,
      'sequential': false,
      'selected': tracks,
    });
    expect(startedFacts(session.copyWith(selected: null))['selected'], isNull);
  });

  test('a failure keeps the server status and code when it has them', () {
    expect(
      failedFacts(
        'update',
        ApiException('boom', status: 502, code: 'upstream_error'),
      ),
      <String, Object?>{
        'request': 'update',
        'status': 502,
        'code': 'upstream_error',
        'message': 'boom',
      },
    );
    expect(failedFacts('start', StateError('lost')), <String, Object?>{
      'request': 'start',
      'message': 'Bad state: lost',
    });
  });
}
