import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/job/job.dart';

part 'version_job.freezed.dart';
part 'version_job.g.dart';

@freezed
abstract class VersionJob with _$VersionJob {
  const factory VersionJob({
    required String id,
    required JobKind kind,
    required JobStatus status,
    int? ahead,
    required String createdAt,
    String? startedAt,
    String? finishedAt,
    int? elapsedMs,
    String? lastError,
    String? language,
    String? subtitleId,
  }) = _VersionJob;

  factory VersionJob.fromJson(Map<String, dynamic> json) =>
      _$VersionJobFromJson(json);
}

extension VersionJobState on VersionJob {
  bool get active => status == JobStatus.queued || status == JobStatus.running;
}
