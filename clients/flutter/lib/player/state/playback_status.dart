import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/player/state/playback_life.dart';
import 'package:shadowmask/player/state/playback_notice.dart';
import 'package:shadowmask/player/state/sent_change.dart';

part 'playback_status.freezed.dart';

@freezed
abstract class PlaybackStatus with _$PlaybackStatus {
  const factory PlaybackStatus({
    @Default(PlaybackLife.open) PlaybackLife life,
    @Default(false) bool gathered,
    SentChange? sent,
    @Default(0) int lastChangeId,
    @Default(false) bool slow,
    @Default(false) bool stalled,
    @Default(false) bool holding,
    String? error,
    PlaybackNotice? notice,
  }) = _PlaybackStatus;
}
