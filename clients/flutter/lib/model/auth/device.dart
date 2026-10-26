import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/auth/device_target.dart';
import 'package:shadowmask/model/auth/shared_with.dart';

part 'device.freezed.dart';
part 'device.g.dart';

@freezed
abstract class Device with _$Device {
  const factory Device({
    required String id,
    required String userId,
    required String name,
    required String platform,
    required String createdAt,
    String? lastSeen,
    DeviceTarget? target,
    @Default(<SharedWith>[]) List<SharedWith> sharedWith,
  }) = _Device;

  factory Device.fromJson(Map<String, dynamic> json) => _$DeviceFromJson(json);
}
