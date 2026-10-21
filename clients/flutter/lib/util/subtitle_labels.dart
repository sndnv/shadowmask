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

String subtitleCandidateName(SubtitleCandidate c) => c.releaseName ?? c.fileId;

String subtitleCandidateDetails(SubtitleCandidate c) => <String>[
  c.language == null ? '—' : languageLabel(c.language!),
  c.format.name,
].join(' · ');

String audioTrackLabel(AudioTrack a) => <String>[
  if (a.language != null) languageLabel(a.language!),
  a.codec,
  '${a.channels}ch',
].join(' · ');

List<String> playerAudioLabels(VersionDetail version) => _numbered(<String>[
  for (final AudioTrack a in version.audio) _audioName(a),
]);

List<String> playerSubtitleLabels(VersionDetail version) => _numbered(<String>[
  for (final SubtitleTrack s in version.subtitles) _trackName(s),
  for (final SubtitleFile f in version.subtitleFiles) _fileName(f),
]);

String _languageName(String? code) =>
    code == null ? Strings.unknownValue : languageLabel(code);

String _audioName(AudioTrack a) => <String>[
  _languageName(a.language),
  if (a.title case final String title) title,
].join(' · ');

String _trackName(SubtitleTrack s) => <String>[
  _languageName(s.language),
  if (s.title case final String title) title,
  if (s.forced) Strings.playerForced,
].join(' · ');

String _fileName(SubtitleFile f) => <String>[
  _languageName(f.language),
  if (f.label case final String label when label.isNotEmpty) label,
  if (f.source == SubtitleSource.machineTranslated) Strings.sourceTranslated,
  if (f.source == SubtitleSource.generated) Strings.sourceGenerated,
].join(' · ');

List<String> _numbered(List<String> labels) {
  final Map<String, int> seen = <String, int>{};
  return <String>[
    for (final String label in labels)
      switch (seen.update(label, (int n) => n + 1, ifAbsent: () => 1)) {
        1 => label,
        final int n => '$label $n',
      },
  ];
}

String downloadCountLabel(int n) => Strings.downloadCount(n, groupedCount(n));
