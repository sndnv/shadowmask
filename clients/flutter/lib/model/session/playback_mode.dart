import 'package:freezed_annotation/freezed_annotation.dart';

enum PlaybackMode {
  @JsonValue('direct')
  direct,
  @JsonValue('remux')
  remux,
  @JsonValue('transcode')
  transcode,
}
