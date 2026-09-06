import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';

part 'playback_session.freezed.dart';
part 'playback_session.g.dart';

@freezed
abstract class PlaybackSession with _$PlaybackSession {
  const factory PlaybackSession({
    required String sessionId,
    @Default(PlaybackMode.direct) PlaybackMode mode,
    @Default('') String manifestUrl,
    @Default(0) int originMs,
    @Default(false) bool sequential,
    @Default(10) int heartbeatIntervalS,
    SelectedTracks? selected,
    Markers? markers,
    @Default(<TrickplayRef>[]) List<TrickplayRef> trickplay,
  }) = _PlaybackSession;

  factory PlaybackSession.fromJson(Map<String, dynamic> json) =>
      _$PlaybackSessionFromJson(json);
}
