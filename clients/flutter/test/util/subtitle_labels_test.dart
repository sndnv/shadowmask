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
    final String first = subtitleFileLabel(_file(label: 'The.Matrix.BluRay'));
    final String second = subtitleFileLabel(_file(label: 'The.Matrix.WEB'));

    expect(first, isNot(second));
    expect(first, contains('The.Matrix.BluRay'));
    expect(second, contains('The.Matrix.WEB'));
  });

  test('a file with no label keeps its old three-part label', () {
    expect(subtitleFileLabel(_file()), 'EN · srt · openSubtitles');
  });

  test('a candidate label still leads with the release name', () {
    const SubtitleCandidate c = SubtitleCandidate(
      fileId: '42',
      language: 'en',
      releaseName: 'The.Matrix.BluRay',
      format: SubtitleFormat.srt,
      downloadCount: 10,
    );
    expect(subtitleCandidateLabel(c), startsWith('The.Matrix.BluRay'));
  });
}
