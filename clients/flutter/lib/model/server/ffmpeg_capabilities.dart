import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/server/ffmpeg_component.dart';

part 'ffmpeg_capabilities.freezed.dart';
part 'ffmpeg_capabilities.g.dart';

@freezed
abstract class FfmpegCapabilities with _$FfmpegCapabilities {
  const factory FfmpegCapabilities({
    String? version,
    String? error,
    @Default(<String>[]) List<String> hwaccels,
    @Default(<FfmpegComponent>[]) List<FfmpegComponent> encoders,
    @Default(<FfmpegComponent>[]) List<FfmpegComponent> filters,
  }) = _FfmpegCapabilities;

  factory FfmpegCapabilities.fromJson(Map<String, dynamic> json) =>
      _$FfmpegCapabilitiesFromJson(json);
}
