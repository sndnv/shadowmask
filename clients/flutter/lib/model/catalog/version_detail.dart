import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';

part 'version_detail.freezed.dart';
part 'version_detail.g.dart';

enum HdrFormat {
  @JsonValue('hdr10')
  hdr10,
  @JsonValue('hdr10_plus')
  hdr10Plus,
  @JsonValue('dolby_vision')
  dolbyVision,
  @JsonValue('hlg')
  hlg,
}

enum SubtitleFormat {
  @JsonValue('srt')
  srt,
  @JsonValue('ass')
  ass,
  @JsonValue('vtt')
  vtt,
  @JsonValue('pgs')
  pgs,
  @JsonValue('vob_sub')
  vobSub,
}

enum SubtitleSource {
  @JsonValue('open_subtitles')
  openSubtitles,
  @JsonValue('external')
  external,
  @JsonValue('generated')
  generated,
  @JsonValue('machine_translated')
  machineTranslated,
  @JsonValue('combined')
  combined,
}

@freezed
abstract class VideoTrack with _$VideoTrack {
  const factory VideoTrack({
    required int index,
    required String codec,
    @Default(0) int width,
    @Default(0) int height,
    @Default(8) int bitDepth,
    HdrFormat? hdr,
    @Default(0) double frameRate,
    int? bitrate,
  }) = _VideoTrack;

  factory VideoTrack.fromJson(Map<String, dynamic> json) =>
      _$VideoTrackFromJson(json);
}

@freezed
abstract class AudioTrack with _$AudioTrack {
  const factory AudioTrack({
    required int index,
    required String codec,
    @Default(2) int channels,
    String? language,
    int? bitrate,
  }) = _AudioTrack;

  factory AudioTrack.fromJson(Map<String, dynamic> json) =>
      _$AudioTrackFromJson(json);
}

@freezed
abstract class SubtitleTrack with _$SubtitleTrack {
  const factory SubtitleTrack({
    required int index,
    String? language,
    required SubtitleFormat format,
    @Default(false) bool forced,
    @JsonKey(name: 'default') @Default(false) bool isDefault,
  }) = _SubtitleTrack;

  factory SubtitleTrack.fromJson(Map<String, dynamic> json) =>
      _$SubtitleTrackFromJson(json);
}

@freezed
abstract class SubtitleFile with _$SubtitleFile {
  const factory SubtitleFile({
    required String id,
    String? language,
    required SubtitleFormat format,
    required SubtitleSource source,
    String? label,
    @Default(false) bool pinned,
  }) = _SubtitleFile;

  factory SubtitleFile.fromJson(Map<String, dynamic> json) =>
      _$SubtitleFileFromJson(json);
}

@freezed
abstract class Chapter with _$Chapter {
  const factory Chapter({required String title, @Default(0) int startMs}) =
      _Chapter;

  factory Chapter.fromJson(Map<String, dynamic> json) =>
      _$ChapterFromJson(json);
}

@freezed
abstract class Marker with _$Marker {
  const factory Marker({@Default(0) int startMs, @Default(0) int endMs}) =
      _Marker;

  factory Marker.fromJson(Map<String, dynamic> json) => _$MarkerFromJson(json);
}

@freezed
abstract class Markers with _$Markers {
  const factory Markers({
    @Default(<Marker>[]) List<Marker> intro,
    @Default(<Marker>[]) List<Marker> credits,
  }) = _Markers;

  factory Markers.fromJson(Map<String, dynamic> json) =>
      _$MarkersFromJson(json);
}

@freezed
abstract class TrickplayRef with _$TrickplayRef {
  const factory TrickplayRef({
    @Default(0) int intervalMs,
    @Default(0) int columns,
    @Default(0) int rows,
    @Default(0) int tileWidth,
    @Default(0) int tileHeight,
    @Default(0) int sheets,
  }) = _TrickplayRef;

  factory TrickplayRef.fromJson(Map<String, dynamic> json) =>
      _$TrickplayRefFromJson(json);
}

@freezed
abstract class VersionDetail with _$VersionDetail {
  const factory VersionDetail({
    required String id,
    required TitleRef title,
    required String libraryId,
    required Quality quality,
    required String container,
    @Default(0) int sizeBytes,
    @Default(0) int durationMs,
    @Default(true) bool available,
    String? addedAt,
    String? updatedAt,
    String? path,
    @Default(<VideoTrack>[]) List<VideoTrack> video,
    @Default(<AudioTrack>[]) List<AudioTrack> audio,
    @Default(<SubtitleTrack>[]) List<SubtitleTrack> subtitles,
    @Default(<SubtitleFile>[]) List<SubtitleFile> subtitleFiles,
    @Default(<Chapter>[]) List<Chapter> chapters,
    Markers? markers,
    @Default(<TrickplayRef>[]) List<TrickplayRef> trickplay,
  }) = _VersionDetail;

  factory VersionDetail.fromJson(Map<String, dynamic> json) =>
      _$VersionDetailFromJson(json);
}
