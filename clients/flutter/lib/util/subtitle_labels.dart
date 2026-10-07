import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/subtitle_candidate.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/util/languages.dart';

String subtitleSourceLabel(SubtitleSource source) => switch (source) {
  SubtitleSource.openSubtitles => Strings.sourceOpenSubtitles,
  SubtitleSource.external => Strings.sourceExternal,
  SubtitleSource.generated => Strings.sourceGenerated,
  SubtitleSource.machineTranslated => Strings.sourceTranslated,
  SubtitleSource.combined => Strings.sourceCombined,
};

String subtitleFileLabel(SubtitleFile s) => <String>[
  s.language == null ? '—' : languageLabel(s.language!),
  s.format.name,
  subtitleSourceLabel(s.source),
  if (s.label != null) s.label!,
].join(' · ');

String subtitleCandidateLabel(SubtitleCandidate c) => <String>[
  c.releaseName ?? c.fileId,
  c.language == null ? '—' : languageLabel(c.language!),
  c.format.name,
].join(' · ');

String audioTrackLabel(AudioTrack a) => <String>[
  if (a.language != null) languageLabel(a.language!),
  a.codec,
  '${a.channels}ch',
].join(' · ');

String downloadCountLabel(int n) => Strings.downloadCount(n, groupedCount(n));
