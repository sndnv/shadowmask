import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';

Map<String, dynamic> _version({
  List<Map<String, dynamic>> audio = const <Map<String, dynamic>>[],
  List<Map<String, dynamic>> subtitles = const <Map<String, dynamic>>[],
  List<Map<String, dynamic>> files = const <Map<String, dynamic>>[],
}) => <String, dynamic>{
  'id': 'v1',
  'title': <String, dynamic>{'type': 'movie', 'id': 't1'},
  'library_id': 'lib',
  'quality': 'hd',
  'container': 'mkv',
  'audio': audio,
  'subtitles': subtitles,
  'subtitle_files': files,
};

void main() {
  test('a DVB track is read as an image subtitle', () {
    final VersionDetail version = VersionDetail.fromJson(
      _version(
        subtitles: <Map<String, dynamic>>[
          <String, dynamic>{'index': 3, 'format': 'dvb'},
        ],
      ),
    );

    expect(version.subtitles.single.format, SubtitleFormat.dvb);
    expect(version.subtitles.single.format.isImage, isTrue);
  });

  test('a subtitle format this build does not know still loads the '
      'version', () {
    final VersionDetail version = VersionDetail.fromJson(
      _version(
        subtitles: <Map<String, dynamic>>[
          <String, dynamic>{'index': 3, 'format': 'teletext'},
        ],
        files: <Map<String, dynamic>>[
          <String, dynamic>{'id': 'f1', 'format': 'ttml', 'source': 'external'},
        ],
      ),
    );

    expect(version.subtitles.single.format, SubtitleFormat.unknown);
    expect(version.subtitleFiles.single.format, SubtitleFormat.unknown);
    expect(SubtitleFormat.unknown.isImage, isFalse);
  });

  test('only bitmap formats are image subtitles', () {
    expect(
      SubtitleFormat.values.where((SubtitleFormat f) => f.isImage).toList(),
      <SubtitleFormat>[
        SubtitleFormat.pgs,
        SubtitleFormat.vobSub,
        SubtitleFormat.dvb,
      ],
    );
  });

  test('tracks carry their own titles', () {
    final VersionDetail version = VersionDetail.fromJson(
      _version(
        audio: <Map<String, dynamic>>[
          <String, dynamic>{'index': 1, 'codec': 'ac3', 'title': 'Surround'},
        ],
        subtitles: <Map<String, dynamic>>[
          <String, dynamic>{'index': 3, 'format': 'srt', 'title': 'SDH'},
        ],
      ),
    );

    expect(version.audio.single.title, 'Surround');
    expect(version.subtitles.single.title, 'SDH');
  });
}
