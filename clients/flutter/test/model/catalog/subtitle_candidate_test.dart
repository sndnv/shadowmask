import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/subtitle_candidate.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';

void main() {
  test('SubtitleCandidate parses snake_case fields', () {
    final SubtitleCandidate c = SubtitleCandidate.fromJson(<String, dynamic>{
      'file_id': 'f1',
      'language': 'eng',
      'release_name': 'BluRay.x264',
      'format': 'srt',
      'download_count': 1200,
      'rating': 8.5,
    });

    expect(c.fileId, 'f1');
    expect(c.language, 'eng');
    expect(c.releaseName, 'BluRay.x264');
    expect(c.format, SubtitleFormat.srt);
    expect(c.downloadCount, 1200);
    expect(c.rating, 8.5);
  });

  test('SubtitleCandidate tolerates missing optionals', () {
    final SubtitleCandidate c = SubtitleCandidate.fromJson(<String, dynamic>{
      'file_id': 'f2',
      'format': 'ass',
    });

    expect(c.language, isNull);
    expect(c.releaseName, isNull);
    expect(c.downloadCount, isNull);
    expect(c.rating, isNull);
    expect(c.format, SubtitleFormat.ass);
  });
}
