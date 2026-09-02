import 'package:shadowmask/model/job/job.dart';

String jobKindLabel(JobKind kind) => switch (kind) {
  JobKind.libraryScan => 'Library scan',
  JobKind.metadata => 'Metadata',
  JobKind.artwork => 'Artwork',
  JobKind.subtitles => 'Subtitles',
  JobKind.trickplay => 'Trickplay',
  JobKind.fingerprint => 'Fingerprint',
  JobKind.dedup => 'Dedup',
  JobKind.cacheEviction => 'Cache eviction',
  JobKind.searchReindex => 'Search reindex',
  JobKind.ingest => 'Ingest',
  JobKind.relink => 'Relink',
  JobKind.transcription => 'Transcription',
  JobKind.translation => 'Translation',
  JobKind.upscale => 'Upscale',
  JobKind.combine => 'Combine',
  JobKind.fetch => 'Fetch',
  JobKind.scheduledScan => 'Scheduled scan',
  JobKind.retention => 'Retention',
  JobKind.orphanSweep => 'Orphan sweep',
};

String shortJobId(String id) => id.split('-').first;

String jobPriorityLabel(JobPriority priority) => switch (priority) {
  JobPriority.low => 'Low',
  JobPriority.normal => 'Normal',
  JobPriority.high => 'High',
};
