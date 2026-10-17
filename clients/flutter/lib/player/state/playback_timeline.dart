import 'package:freezed_annotation/freezed_annotation.dart';

part 'playback_timeline.freezed.dart';

@freezed
abstract class PlaybackTimeline with _$PlaybackTimeline {
  const factory PlaybackTimeline({
    @Default(0) int positionMs,
    int? seekingToMs,
    @Default(0) int reachMs,
    @Default(0) int bufferedAheadMs,
    @Default(false) bool playing,
    @Default(false) bool ended,
    @Default(false) bool waiting,
    @Default(0) double bufferingPercent,
  }) = _PlaybackTimeline;
}
