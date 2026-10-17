import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_life.dart';
import 'package:shadowmask/player/state/playback_status.dart';
import 'package:shadowmask/player/state/playback_stream.dart';
import 'package:shadowmask/player/state/playback_timeline.dart';
import 'package:shadowmask/player/state/playback_title.dart';
import 'package:shadowmask/view/playback_controls.dart';

part 'playback_state.freezed.dart';

@freezed
abstract class PlaybackState with _$PlaybackState {
  const PlaybackState._();

  const factory PlaybackState({
    required String versionId,
    @Default(PlaybackTitle()) PlaybackTitle title,
    @Default(PlaybackTimeline()) PlaybackTimeline timeline,
    @Default(LocalSettings()) LocalSettings local,
    @Default(PlaybackControls()) PlaybackControls remote,
    PlaybackStream? stream,
    @Default(PlaybackStatus()) PlaybackStatus status,
  }) = _PlaybackState;

  bool get watching =>
      status.life == PlaybackLife.open && status.sent == null && stream != null;

  bool get reportsPlaying =>
      timeline.playing && status.sent == null && !status.holding;
}
