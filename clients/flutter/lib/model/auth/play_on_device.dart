import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/auth/device.dart';
import 'package:shadowmask/model/auth/device_target.dart';

part 'play_on_device.freezed.dart';
part 'play_on_device.g.dart';

@freezed
abstract class PlayOnDevice with _$PlayOnDevice {
  const factory PlayOnDevice({
    required String id,
    required String name,
    String? owner,
    DeviceTarget? target,
  }) = _PlayOnDevice;

  factory PlayOnDevice.own(Device device) =>
      PlayOnDevice(id: device.id, name: device.name, target: device.target);

  factory PlayOnDevice.fromJson(Map<String, dynamic> json) =>
      _$PlayOnDeviceFromJson(json);
}
