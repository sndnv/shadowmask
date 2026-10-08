import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/server/hardware_test.dart';

part 'hardware_capabilities.freezed.dart';
part 'hardware_capabilities.g.dart';

@freezed
abstract class HardwareCapabilities with _$HardwareCapabilities {
  const factory HardwareCapabilities({
    required String mode,
    required String device,
    @Default(false) bool devicePresent,
    @Default(false) bool inUse,
    required HardwareTest test,
  }) = _HardwareCapabilities;

  factory HardwareCapabilities.fromJson(Map<String, dynamic> json) =>
      _$HardwareCapabilitiesFromJson(json);
}
