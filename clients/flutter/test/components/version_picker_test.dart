import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/version_picker.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';

void main() {
  test('distinct subtitle languages deduplicate per language', () {
    final VersionDetail d = VersionDetail.fromJson(<String, dynamic>{
      'id': 'v1',
      'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      'library_id': 'l',
      'quality': 'fhd',
      'container': 'mkv',
      'subtitles': <dynamic>[
        <String, dynamic>{
          'index': 0,
          'language': 'en',
          'format': 'srt',
          'forced': false,
          'default': true,
        },
        <String, dynamic>{
          'index': 1,
          'language': 'en',
          'format': 'srt',
          'forced': true,
          'default': false,
        },
        <String, dynamic>{
          'index': 2,
          'language': 'es',
          'format': 'srt',
          'forced': false,
          'default': false,
        },
      ],
      'subtitle_files': <dynamic>[
        <String, dynamic>{
          'id': 'sf',
          'language': 'en',
          'format': 'srt',
          'source': 'external',
        },
      ],
    });
    expect(distinctSubtitleLanguages(d), 2);
  });
}
