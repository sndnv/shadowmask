import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/server/ffmpeg_capabilities.dart';
import 'package:shadowmask/model/server/hardware_capabilities.dart';
import 'package:shadowmask/model/server/host_capabilities.dart';

part 'capabilities_report.freezed.dart';
part 'capabilities_report.g.dart';

enum CheckState {
  @JsonValue('unchecked')
  unchecked,
  @JsonValue('checking')
  checking,
  @JsonValue('ready')
  ready,
}

@freezed
abstract class CapabilitiesReport with _$CapabilitiesReport {
  const factory CapabilitiesReport({
    required CheckState state,
    String? checkedAt,
    HostCapabilities? host,
    FfmpegCapabilities? ffmpeg,
    HardwareCapabilities? hardware,
  }) = _CapabilitiesReport;

  factory CapabilitiesReport.fromJson(Map<String, dynamic> json) =>
      _$CapabilitiesReportFromJson(json);
}
