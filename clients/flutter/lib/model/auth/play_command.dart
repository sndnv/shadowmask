import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/auth/device_target.dart';

part 'play_command.freezed.dart';
part 'play_command.g.dart';

@freezed
abstract class PlayCommand with _$PlayCommand {
  const factory PlayCommand({
    required String token,
    required String expiresAt,
    required DeviceTarget target,
  }) = _PlayCommand;

  factory PlayCommand.fromJson(Map<String, dynamic> json) =>
      _$PlayCommandFromJson(json);
}
