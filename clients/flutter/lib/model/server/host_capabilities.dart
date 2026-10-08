import 'package:freezed_annotation/freezed_annotation.dart';

part 'host_capabilities.freezed.dart';
part 'host_capabilities.g.dart';

@freezed
abstract class HostCapabilities with _$HostCapabilities {
  const factory HostCapabilities({
    String? cpu,
    int? cores,
    @Default(0) int threads,
  }) = _HostCapabilities;

  factory HostCapabilities.fromJson(Map<String, dynamic> json) =>
      _$HostCapabilitiesFromJson(json);
}
