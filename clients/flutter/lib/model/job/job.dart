import 'package:freezed_annotation/freezed_annotation.dart';

part 'job.freezed.dart';
part 'job.g.dart';

enum JobKind {
  @JsonValue('library_scan')
  libraryScan,
  @JsonValue('metadata')
  metadata,
  @JsonValue('artwork')
  artwork,
  @JsonValue('subtitles')
  subtitles,
  @JsonValue('trickplay')
  trickplay,
  @JsonValue('fingerprint')
  fingerprint,
  @JsonValue('dedup')
  dedup,
  @JsonValue('cache_eviction')
  cacheEviction,
  @JsonValue('search_reindex')
  searchReindex,
  @JsonValue('ingest')
  ingest,
  @JsonValue('relink')
  relink,
  @JsonValue('transcription')
  transcription,
  @JsonValue('translation')
  translation,
  @JsonValue('upscale')
  upscale,
  @JsonValue('combine')
  combine,
  @JsonValue('fetch')
  fetch,
  @JsonValue('scheduled_scan')
  scheduledScan,
  @JsonValue('retention')
  retention,
  @JsonValue('orphan_sweep')
  orphanSweep,
}

enum JobStatus {
  @JsonValue('queued')
  queued,
  @JsonValue('running')
  running,
  @JsonValue('succeeded')
  succeeded,
  @JsonValue('failed')
  failed,
  @JsonValue('cancelled')
  cancelled,
}

enum JobPriority {
  @JsonValue('low')
  low,
  @JsonValue('normal')
  normal,
  @JsonValue('high')
  high,
}

@freezed
abstract class Job with _$Job {
  const factory Job({
    required String id,
    required JobKind kind,
    required JobStatus status,
    required JobPriority priority,
    @Default(0) double progress,
    @Default(0) int attempts,
    String? lastError,
    required String createdAt,
    required String updatedAt,
    String? startedAt,
    String? finishedAt,
    String? parentId,
    @Default(false) bool cancellable,
  }) = _Job;

  factory Job.fromJson(Map<String, dynamic> json) => _$JobFromJson(json);
}
