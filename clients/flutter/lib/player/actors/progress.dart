import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_step.dart';
import 'package:shadowmask/player/state/playback_stream.dart';

PlaybackStep withProgress(PlaybackState before, PlaybackStep step) {
  final (PlaybackState after, List<PlaybackCommand> commands) = step;
  final PlaybackStream? stream = after.stream;
  if (stream == null ||
      !after.watching ||
      after.reportsPlaying == before.reportsPlaying) {
    return step;
  }
  return (
    after,
    <PlaybackCommand>[
      ...commands,
      SendProgress(
        sessionId: stream.sessionId,
        positionMs: after.timeline.positionMs,
        playing: after.reportsPlaying,
      ),
    ],
  );
}
