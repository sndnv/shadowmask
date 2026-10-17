import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/view/playback_controls.dart';

part 'sent_change.freezed.dart';

@freezed
sealed class SentChange with _$SentChange {
  const factory SentChange.start({required int id, required int positionMs}) =
      SentStart;

  const factory SentChange.controls({
    required int id,
    required PlaybackControls controls,
    required PlaybackControls previous,
    required int positionMs,
  }) = SentControls;

  const factory SentChange.seek({
    required int id,
    required int positionMs,
    required int previousMs,
    required bool previousPlaying,
  }) = SentSeek;

  const factory SentChange.load({required int id, required int positionMs}) =
      SentLoad;
}
