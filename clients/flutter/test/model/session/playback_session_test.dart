import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/model/session/playback_session.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';

void main() {
  test('PlaybackSession.fromJson reads mode, manifest, heartbeat, tracks', () {
    final PlaybackSession s = PlaybackSession.fromJson(<String, dynamic>{
      'session_id': 's1',
      'mode': 'transcode',
      'manifest_url': '/stream/tok/master.m3u8',
      'heartbeat_interval_s': 7,
      'selected': <String, dynamic>{
        'audio_track': 1,
        'subtitle_track': <String, dynamic>{'type': 'embedded', 'index': 5},
        'subtitle_delivery': 'hls_vtt',
      },
      'trickplay': <dynamic>[
        <String, dynamic>{
          'interval_ms': 10000,
          'columns': 5,
          'rows': 5,
          'tile_width': 320,
          'tile_height': 180,
          'sheets': 2,
        },
      ],
    });
    expect(s.sessionId, 's1');
    expect(s.mode, PlaybackMode.transcode);
    expect(s.manifestUrl, '/stream/tok/master.m3u8');
    expect(s.heartbeatIntervalS, 7);
    expect(s.selected!.audioTrack, 1);
    expect(s.selected!.subtitleTrack!.kind, SubtitleKind.embedded);
    expect(s.selected!.subtitleTrack!.index, 5);
    expect(s.trickplay.single.columns, 5);
  });
}
