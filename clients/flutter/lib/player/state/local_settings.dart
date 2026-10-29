import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_size.dart';

part 'local_settings.freezed.dart';

@freezed
abstract class LocalSettings with _$LocalSettings {
  const factory LocalSettings({
    @Default(1) double speed,
    @Default(1) double volume,
    @Default(false) bool muted,
    @Default(false) bool mutedByPolicy,
    @Default(SubtitleSize.deviceDefault) SubtitleSize subtitleSize,
    @Default(false) bool fullscreen,
    @Default(false) bool pictureInPicture,
    @Default(false) bool wide,
    @Default(false) bool remaining,
    @Default(false) bool diagnostics,
    @Default(kDefaultNetworkTimeoutSeconds) int networkTimeoutS,
    @Default(kDefaultBufferSeconds) int bufferS,
    @Default(kDefaultBufferBytes) int bufferBytes,
    @Default(false) bool waitForBuffer,
    @Default(10) int autoplayNextS,
    @Default(DeliveryPreference.auto) DeliveryPreference defaultDelivery,
    @Default(false) bool defaultDownmix,
    @Default(true) bool defaultNormalize,
    @Default(DialogueLift.none) DialogueLift defaultDialogue,
    @Default(NightMode.off) NightMode defaultNight,
    ClientDecoding? decoding,
  }) = _LocalSettings;
}
