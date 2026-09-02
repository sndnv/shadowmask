import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/resume_card.dart';

part 'now_playing.freezed.dart';
part 'now_playing.g.dart';

@freezed
abstract class NowPlaying with _$NowPlaying {
  const factory NowPlaying({
    required String sessionId,
    required String userId,
    String? deviceId,
    required String versionId,
    @Default(0) int positionMs,
    @Default('') String state,
    required String startedAt,
    required String lastHeartbeatAt,
    ResumeCard? card,
  }) = _NowPlaying;

  factory NowPlaying.fromJson(Map<String, dynamic> json) =>
      _$NowPlayingFromJson(json);
}
