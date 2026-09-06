import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/track_carry.dart';

VersionDetail _version({
  List<AudioTrack> audio = const <AudioTrack>[],
  List<SubtitleTrack> subtitles = const <SubtitleTrack>[],
  List<SubtitleFile> files = const <SubtitleFile>[],
}) => VersionDetail(
  id: 'v1',
  title: const TitleRef(type: TitleKind.episode, id: 'e1'),
  libraryId: 'lib',
  quality: Quality.hd,
  container: 'mkv',
  audio: audio,
  subtitles: subtitles,
  subtitleFiles: files,
);

void main() {
  group('carryTracks', () {
    test('resolves an index to the language it is labelled with', () {
      final VersionDetail v = _version(
        audio: const <AudioTrack>[
          AudioTrack(index: 1, codec: 'aac', language: 'eng'),
          AudioTrack(index: 2, codec: 'aac', language: 'fra'),
        ],
        subtitles: const <SubtitleTrack>[
          SubtitleTrack(index: 3, language: 'fra', format: SubtitleFormat.srt),
        ],
      );
      final PlaybackControls carried = carryTracks(
        const PlaybackControls(
          audioTrack: 2,
          subtitle: SubtitleSelection.embedded(3),
        ),
        v,
      );
      expect(carried.audioLanguage, 'fra');
      expect(carried.subtitleLanguage, 'fra');
      expect(
        carried.audioTrack,
        isNull,
        reason:
            'stream 2 of this file has nothing to do with stream 2 of the next',
      );
      expect(carried.subtitle, isNull);
    });

    test('a sidecar file carries its language, never its id', () {
      final VersionDetail v = _version(
        files: const <SubtitleFile>[
          SubtitleFile(
            id: 'sub-1',
            language: 'nld',
            format: SubtitleFormat.srt,
            source: SubtitleSource.openSubtitles,
          ),
        ],
      );
      final PlaybackControls carried = carryTracks(
        const PlaybackControls(subtitle: SubtitleSelection.file('sub-1')),
        v,
      );
      expect(carried.subtitleLanguage, 'nld');
      expect(carried.toQuery().containsKey('sub'), isFalse);
    });

    test('an unlabelled track carries nothing', () {
      final VersionDetail v = _version(
        audio: const <AudioTrack>[AudioTrack(index: 1, codec: 'aac')],
        subtitles: const <SubtitleTrack>[
          SubtitleTrack(index: 2, format: SubtitleFormat.srt),
        ],
      );
      final PlaybackControls carried = carryTracks(
        const PlaybackControls(
          audioTrack: 1,
          subtitle: SubtitleSelection.embedded(2),
        ),
        v,
      );
      expect(carried.audioLanguage, isNull);
      expect(carried.subtitleLanguage, isNull);
      expect(carried.toQuery(), isEmpty);
    });

    test('a request this episode could not honour still carries forward', () {
      final PlaybackControls carried = carryTracks(
        const PlaybackControls(audioLanguage: 'fra', subtitleLanguage: 'fra'),
        _version(),
      );
      expect(carried.audioLanguage, 'fra');
      expect(carried.subtitleLanguage, 'fra');
    });

    test('subtitles switched off carry as off, never as a language', () {
      final VersionDetail v = _version(
        subtitles: const <SubtitleTrack>[
          SubtitleTrack(index: 2, language: 'eng', format: SubtitleFormat.srt),
        ],
      );
      final PlaybackControls carried = carryTracks(
        const PlaybackControls(
          subtitle: SubtitleSelection.embedded(2),
          subtitleLanguage: 'eng',
          subtitleOff: true,
        ),
        v,
      );
      expect(carried.subtitleOff, isTrue);
      expect(carried.subtitleLanguage, isNull);
      expect(carried.toQuery(), <String, String?>{'soff': '1'});
    });

    test('an index that is not in this version resolves to nothing', () {
      final VersionDetail v = _version(
        audio: const <AudioTrack>[
          AudioTrack(index: 1, codec: 'aac', language: 'eng'),
        ],
        subtitles: const <SubtitleTrack>[
          SubtitleTrack(index: 2, language: 'eng', format: SubtitleFormat.srt),
        ],
        files: const <SubtitleFile>[
          SubtitleFile(
            id: 'sub-1',
            language: 'eng',
            format: SubtitleFormat.srt,
            source: SubtitleSource.external,
          ),
        ],
      );
      expect(audioLanguageOf(v, 9), isNull);
      expect(audioLanguageOf(v, null), isNull);
      expect(
        subtitleLanguageOf(v, const SubtitleSelection.embedded(9)),
        isNull,
      );
      expect(
        subtitleLanguageOf(v, const SubtitleSelection.file('gone')),
        isNull,
      );
      expect(subtitleLanguageOf(v, null), isNull);
    });
  });
}
