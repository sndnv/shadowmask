import 'package:freezed_annotation/freezed_annotation.dart';

part 'player_snapshot.freezed.dart';

@freezed
abstract class PlayerSnapshot with _$PlayerSnapshot {
  const PlayerSnapshot._();

  const factory PlayerSnapshot({
    @Default(0) int positionMs,
    @Default(0) int durationMs,
    @Default(0) int bufferedAheadMs,
    @Default(false) bool playing,
    @Default(false) bool ready,
    @Default(false) bool buffering,
    @Default(0) double bufferingPercent,
    @Default(false) bool ended,
    String? error,
  }) = _PlayerSnapshot;

  double get fraction =>
      durationMs > 0 ? (positionMs / durationMs).clamp(0.0, 1.0) : 0.0;

  bool get waiting => error == null && !ended && (buffering || !ready);

  PlayerSnapshot onTimeline({required int originMs, required int fullMs}) {
    if (originMs == 0) {
      return this;
    }
    return copyWith(
      positionMs: positionMs + originMs,
      durationMs: fullMs > 0 ? fullMs : durationMs + originMs,
    );
  }
}
