import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_stream.dart';
import 'package:shadowmask/view/playback_controls.dart';

part 'playback_command.freezed.dart';

@freezed
sealed class PlaybackCommand with _$PlaybackCommand {
  const factory PlaybackCommand.sendStart({
    required int id,
    required PlaybackControls controls,
    required int positionMs,
  }) = SendStart;

  const factory PlaybackCommand.sendUpdate({
    required int id,
    required String sessionId,
    required PlaybackControls controls,
    required int positionMs,
  }) = SendUpdate;

  const factory PlaybackCommand.sendSeek({
    required int id,
    required String sessionId,
    required int positionMs,
  }) = SendSeek;

  const factory PlaybackCommand.sendProgress({
    required String sessionId,
    required int positionMs,
    required bool playing,
  }) = SendProgress;

  const factory PlaybackCommand.endSession({
    required String sessionId,
    int? positionMs,
    @Default(false) bool playing,
  }) = EndSession;

  const factory PlaybackCommand.loadStream({
    required int id,
    required PlaybackStream stream,
    required int offsetMs,
  }) = LoadStream;

  const factory PlaybackCommand.seekPlayer({required int offsetMs}) =
      SeekPlayer;

  const factory PlaybackCommand.playPlayer() = PlayPlayer;

  const factory PlaybackCommand.pausePlayer() = PausePlayer;

  const factory PlaybackCommand.unloadPlayer() = UnloadPlayer;

  const factory PlaybackCommand.applyLocal(LocalSettings local) = ApplyLocal;

  const factory PlaybackCommand.saveLocal(LocalSettings local) = SaveLocal;
}
