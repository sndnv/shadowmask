import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/session/subtitle_selection.dart';

part 'selected_tracks.freezed.dart';
part 'selected_tracks.g.dart';

@freezed
abstract class SelectedTracks with _$SelectedTracks {
  const factory SelectedTracks({
    int? audioTrack,
    SubtitleSelection? subtitleTrack,
    String? subtitleDelivery,
  }) = _SelectedTracks;

  factory SelectedTracks.fromJson(Map<String, dynamic> json) =>
      _$SelectedTracksFromJson(json);
}
