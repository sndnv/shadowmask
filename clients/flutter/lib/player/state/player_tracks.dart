import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';

part 'player_tracks.freezed.dart';

const String kRenditionSubtitles = 'hls_vtt';
const String kEmbeddedDelivery = 'embedded';
const String kExternalSubtitles = 'external';
const String kImageSubtitles = 'image';

@freezed
abstract class PlayerTracks with _$PlayerTracks {
  const factory PlayerTracks({
    @Default(false) bool rendition,
    int? subtitleIndex,
    @Default(false) bool subtitleImage,
    String? subtitleUrl,
    int? audioIndex,
  }) = _PlayerTracks;
}

PlayerTracks playerTracks(SelectedTracks? selected, VersionDetail? version) {
  if (selected == null) {
    return const PlayerTracks();
  }
  final String? delivery = selected.subtitleDelivery;
  final SubtitleSelection? subtitle = selected.subtitleTrack;
  final int? embedded =
      delivery == kEmbeddedDelivery && subtitle?.kind == SubtitleKind.embedded
      ? subtitle?.index
      : null;
  return PlayerTracks(
    rendition: delivery == kRenditionSubtitles,
    subtitleIndex: embedded,
    subtitleImage: embedded != null && _isImage(version, embedded),
    subtitleUrl: delivery == kExternalSubtitles ? selected.subtitleUrl : null,
    audioIndex: selected.audioDelivery == kEmbeddedDelivery
        ? selected.audioTrack
        : null,
  );
}

bool _isImage(VersionDetail? version, int index) =>
    version?.subtitles.any(
      (SubtitleTrack s) => s.index == index && s.format.isImage,
    ) ??
    false;
