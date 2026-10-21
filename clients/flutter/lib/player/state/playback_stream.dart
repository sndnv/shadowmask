import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';

part 'playback_stream.freezed.dart';

@freezed
abstract class PlaybackStream with _$PlaybackStream {
  const factory PlaybackStream({
    required String sessionId,
    required String manifestUrl,
    @Default(0) int originMs,
    required bool sequential,
    @Default(PlaybackMode.direct) PlaybackMode mode,
    String? container,
    SelectedTracks? selected,
    required int heartbeatIntervalS,
    required bool seeksInStream,
    required List<TrickplayRef> trickplay,
  }) = _PlaybackStream;
}
