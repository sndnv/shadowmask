import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/model/job/version_job.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/util/job_labels.dart';
import 'package:shadowmask/util/languages.dart';
import 'package:shadowmask/util/subtitle_labels.dart';

const Set<JobKind> kSubtitleWorkKinds = <JobKind>{
  JobKind.transcription,
  JobKind.translation,
  JobKind.combine,
};

List<VersionJob> subtitleWork(List<VersionJob> jobs) =>
    jobs.where((VersionJob j) => kSubtitleWorkKinds.contains(j.kind)).toList();

String versionJobTitle(VersionJob job) {
  final String? language = job.language;
  final String kind = jobKindLabel(job.kind);
  return language == null ? kind : '$kind · ${languageLabel(language)}';
}

String versionJobStatus(VersionJob job, VersionDetail? detail) =>
    switch (job.status) {
      JobStatus.queued => Strings.jobQueued(job.ahead),
      JobStatus.running => Strings.jobRunning(elapsedText(job.elapsedMs ?? 0)),
      JobStatus.succeeded => _finished(job, detail),
      JobStatus.failed => Strings.jobFailed(jobFailureReason(job.lastError)),
      JobStatus.cancelled => Strings.statusCancelled,
    };

String _finished(VersionJob job, VersionDetail? detail) {
  final String? id = job.subtitleId;
  if (id == null || detail == null) {
    return Strings.statusSucceeded;
  }
  for (final SubtitleFile file in detail.subtitleFiles) {
    if (file.id == id) {
      final String? language = job.language ?? file.language;
      return Strings.jobReady(
        Strings.producedSubtitle(
          language == null ? null : languageLabel(language),
          subtitleSourceLabel(file.source).toLowerCase(),
        ),
      );
    }
  }
  return Strings.jobNoSubtitle;
}
