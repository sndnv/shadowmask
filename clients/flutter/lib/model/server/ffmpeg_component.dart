import 'package:freezed_annotation/freezed_annotation.dart';

part 'ffmpeg_component.freezed.dart';
part 'ffmpeg_component.g.dart';

@freezed
abstract class FfmpegComponent with _$FfmpegComponent {
  const factory FfmpegComponent({
    required String name,
    @Default(false) bool present,
  }) = _FfmpegComponent;

  factory FfmpegComponent.fromJson(Map<String, dynamic> json) =>
      _$FfmpegComponentFromJson(json);
}
