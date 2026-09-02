import 'package:shadowmask/model/catalog/subtitle_candidate.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/util/languages.dart';

String subtitleFileLabel(SubtitleFile s) => <String>[
  s.language == null ? '—' : languageLabel(s.language!),
  s.format.name,
  s.source.name,
  if (s.label != null) s.label!,
].join(' · ');

String subtitleCandidateLabel(SubtitleCandidate c) {
  final List<String> parts = <String>[
    c.releaseName ?? c.fileId,
    c.language == null ? '—' : languageLabel(c.language!),
    c.format.name,
    if (c.downloadCount != null) '↓${c.downloadCount}',
  ];
  return parts.join(' · ');
}
