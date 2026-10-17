import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_step.dart';
import 'package:shadowmask/player/state/playback_stream.dart';

PlaybackCommand endOf(PlaybackState s, PlaybackStream stream) => EndSession(
  sessionId: stream.sessionId,
  positionMs: s.timeline.positionMs,
  playing: s.reportsPlaying,
);

PlaybackStep streamFails(
  PlaybackState s,
  PlaybackStream stream,
  String message,
) => (
  s.copyWith(
    stream: null,
    status: s.status.copyWith(
      sent: null,
      slow: false,
      holding: false,
      error: message,
    ),
  ),
  <PlaybackCommand>[
    EndSession(sessionId: stream.sessionId, positionMs: s.timeline.positionMs),
    const UnloadPlayer(),
  ],
);
