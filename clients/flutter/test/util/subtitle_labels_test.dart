import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/subtitle_candidate.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/util/subtitle_labels.dart';

SubtitleFile _file({String? label}) => SubtitleFile(
  id: 'opensubtitles:v1:42',
  language: 'en',
  format: SubtitleFormat.srt,
  source: SubtitleSource.openSubtitles,
  label: label,
);

void main() {
  test('two files of the same language read differently', () {
    final String first = subtitleFileLabel(_file(label: 'Paper.Skies.BluRay'));
    final String second = subtitleFileLabel(_file(label: 'Paper.Skies.WEB'));

    expect(first, isNot(second));
    expect(first, contains('Paper.Skies.BluRay'));
    expect(second, contains('Paper.Skies.WEB'));
  });

  test('a file with no label reads its provenance, not the enum', () {
    expect(subtitleFileLabel(_file()), 'English · srt · OpenSubtitles');
  });

  test('every provenance has a name a viewer would recognise', () {
    expect(SubtitleSource.values.map(subtitleSourceLabel).toList(), <String>[
      'OpenSubtitles',
      'External',
      'Generated',
      'Translated',
      'Combined',
    ]);
  });

  test('no provenance falls through to the Dart identifier', () {
    for (final SubtitleSource s in SubtitleSource.values) {
      expect(subtitleSourceLabel(s), isNot(s.name));
    }
  });

  test('a candidate label still leads with the release name', () {
    const SubtitleCandidate c = SubtitleCandidate(
      fileId: '42',
      language: 'en',
      releaseName: 'Paper.Skies.BluRay',
      format: SubtitleFormat.srt,
      downloadCount: 10,
    );
    expect(subtitleCandidateLabel(c), startsWith('Paper.Skies.BluRay'));
    expect(subtitleCandidateLabel(c), isNot(contains('10')));
  });

  test('a download count reads in words, grouped', () {
    expect(downloadCountLabel(1), '1 download');
    expect(downloadCountLabel(254311), '254,311 downloads');
  });
}
