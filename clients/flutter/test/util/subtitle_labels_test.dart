import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/subtitle_candidate.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
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

  test('a candidate is named by its release, its details apart', () {
    const SubtitleCandidate c = SubtitleCandidate(
      fileId: '42',
      language: 'en',
      releaseName: 'Paper.Skies.BluRay',
      format: SubtitleFormat.srt,
      downloadCount: 10,
    );
    expect(subtitleCandidateName(c), 'Paper.Skies.BluRay');
    expect(subtitleCandidateDetails(c), 'English · srt');
  });

  test('a candidate with no release name is named by its file id', () {
    const SubtitleCandidate c = SubtitleCandidate(
      fileId: '42',
      format: SubtitleFormat.ass,
    );
    expect(subtitleCandidateName(c), '42');
    expect(subtitleCandidateDetails(c), '— · ass');
  });

  test('a download count reads in words, grouped', () {
    expect(downloadCountLabel(1), '1 download');
    expect(downloadCountLabel(254311), '254,311 downloads');
  });

  test('the player lists embedded tracks then files, by language and their '
      'own names', () {
    const VersionDetail version = VersionDetail(
      id: 'v1',
      title: TitleRef(type: TitleKind.movie, id: 't1'),
      libraryId: 'lib',
      quality: Quality.hd,
      container: 'mkv',
      subtitles: <SubtitleTrack>[
        SubtitleTrack(index: 2, language: 'en', format: SubtitleFormat.srt),
        SubtitleTrack(
          index: 3,
          language: 'en',
          format: SubtitleFormat.pgs,
          title: 'SDH',
        ),
        SubtitleTrack(
          index: 4,
          language: 'fr',
          format: SubtitleFormat.srt,
          forced: true,
        ),
      ],
      subtitleFiles: <SubtitleFile>[
        SubtitleFile(
          id: 'f1',
          language: 'en',
          format: SubtitleFormat.srt,
          source: SubtitleSource.openSubtitles,
          label: 'Paper.Skies.BluRay',
        ),
        SubtitleFile(
          id: 'f2',
          language: 'es',
          format: SubtitleFormat.srt,
          source: SubtitleSource.machineTranslated,
        ),
        SubtitleFile(
          id: 'f3',
          language: 'de',
          format: SubtitleFormat.vtt,
          source: SubtitleSource.generated,
        ),
        SubtitleFile(
          id: 'f4',
          language: 'en+fr',
          format: SubtitleFormat.srt,
          source: SubtitleSource.combined,
        ),
        SubtitleFile(
          id: 'f5',
          format: SubtitleFormat.srt,
          source: SubtitleSource.external,
        ),
      ],
    );

    expect(playerSubtitleLabels(version), <String>[
      'English',
      'English · SDH',
      'French · Forced',
      'English · Paper.Skies.BluRay',
      'Spanish · Translated',
      'German · Generated',
      'English + French',
      'Unknown',
    ]);
  });

  test('entries that would read the same are numbered from the second', () {
    const VersionDetail version = VersionDetail(
      id: 'v1',
      title: TitleRef(type: TitleKind.movie, id: 't1'),
      libraryId: 'lib',
      quality: Quality.hd,
      container: 'mkv',
      audio: <AudioTrack>[
        AudioTrack(index: 1, codec: 'truehd', channels: 8, language: 'en'),
        AudioTrack(index: 2, codec: 'ac3', channels: 6, language: 'en'),
        AudioTrack(index: 3, codec: 'aac', language: 'en', title: 'Commentary'),
        AudioTrack(index: 4, codec: 'ac3', channels: 6, language: 'en'),
      ],
      subtitles: <SubtitleTrack>[
        SubtitleTrack(index: 5, language: 'en', format: SubtitleFormat.srt),
      ],
      subtitleFiles: <SubtitleFile>[
        SubtitleFile(
          id: 'f1',
          language: 'en',
          format: SubtitleFormat.srt,
          source: SubtitleSource.external,
        ),
      ],
    );

    expect(playerAudioLabels(version), <String>[
      'English',
      'English 2',
      'English · Commentary',
      'English 3',
    ]);
    expect(playerSubtitleLabels(version), <String>['English', 'English 2']);
  });
}
