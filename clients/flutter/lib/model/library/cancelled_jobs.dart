import 'package:freezed_annotation/freezed_annotation.dart';

part 'cancelled_jobs.freezed.dart';
part 'cancelled_jobs.g.dart';

@freezed
abstract class CancelledJobs with _$CancelledJobs {
  const factory CancelledJobs({
    @Default(0) int cancelled,
    @Default(0) int signalled,
    @Default(0) int running,
  }) = _CancelledJobs;

  factory CancelledJobs.fromJson(Map<String, dynamic> json) =>
      _$CancelledJobsFromJson(json);
}
