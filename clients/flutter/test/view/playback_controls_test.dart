import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/view/playback_controls.dart';

void main() {
  group('PlaybackControls', () {
    test('builds a full /update body', () {
      final Map<String, dynamic> body = const PlaybackControls(
        height: 720,
        burn: true,
        downmix: true,
        audioTrack: 1,
        subtitle: SubtitleSelection.embedded(2),
        offsetMs: -250,
      ).toUpdateBody();
      expect(body['target_height'], 720);
      expect(body['force_burn'], true);
      expect(body['downmix_stereo'], true);
      expect(body['audio_track'], 1);
      expect(body['subtitle'], <String, dynamic>{
        'action': 'set',
        'track': <String, dynamic>{'type': 'embedded', 'index': 2},
        'offset_ms': -250,
      });
    });

    test('disables subtitles and omits audio when unset', () {
      final Map<String, dynamic> body = const PlaybackControls().toUpdateBody();
      expect(body['target_height'], isNull);
      expect(body['subtitle'], <String, dynamic>{'action': 'disable'});
      expect(body.containsKey('audio_track'), isFalse);
    });

    test('query round trips', () {
      final PlaybackControls c = PlaybackControls.fromQuery(<String, String>{
        'height': '480',
        'burn': '1',
        'audio': '3',
        'sub': 'file:z',
        'offset': '100',
      });
      expect(c.height, 480);
      expect(c.burn, isTrue);
      expect(c.downmix, isFalse);
      expect(c.audioTrack, 3);
      expect(c.subtitle!.toWire(), 'file:z');
      expect(c.offsetMs, 100);
      expect(c.toQuery(), <String, String?>{
        'height': '480',
        'burn': '1',
        'audio': '3',
        'sub': 'file:z',
        'offset': '100',
      });
    });

    test('a carried language round trips through the query', () {
      final PlaybackControls c = PlaybackControls.fromQuery(<String, String>{
        'alang': 'fra',
        'slang': 'eng',
      });
      expect(c.audioLanguage, 'fra');
      expect(c.subtitleLanguage, 'eng');
      expect(c.audioTrack, isNull);
      expect(c.subtitle, isNull);
      expect(c.toQuery(), <String, String?>{'alang': 'fra', 'slang': 'eng'});
    });

    test('subtitles switched off round trip as a choice, not an absence', () {
      final PlaybackControls c = PlaybackControls.fromQuery(<String, String>{
        'soff': '1',
      });
      expect(c.subtitleOff, isTrue);
      expect(c.hasTrackRequest, isTrue);
      expect(c.toStartBody()['subtitle_off'], true);
      expect(c.toQuery(), <String, String?>{'soff': '1'});
    });

    test('the default carries no query and is default', () {
      expect(const PlaybackControls().toQuery(), isEmpty);
      expect(const PlaybackControls().isDefault, isTrue);
      expect(const PlaybackControls().hasTrackRequest, isFalse);
      expect(const PlaybackControls().toStartBody(), isEmpty);
    });

    test('builds a /sessions body from an index or a language, not both', () {
      expect(
        const PlaybackControls(
          height: 720,
          burn: true,
          downmix: true,
          audioTrack: 1,
          subtitle: SubtitleSelection.file('s9'),
        ).toStartBody(),
        <String, dynamic>{
          'target_height': 720,
          'force_burn': true,
          'downmix_stereo': true,
          'audio_track': 1,
          'subtitle': <String, dynamic>{'type': 'file', 'id': 's9'},
        },
      );
      expect(
        const PlaybackControls(
          audioLanguage: 'fra',
          subtitleLanguage: 'fra',
        ).toStartBody(),
        <String, dynamic>{'audio_language': 'fra', 'subtitle_language': 'fra'},
      );
    });

    test('the negotiated selection fills the indices and keeps the carry', () {
      const PlaybackControls asked = PlaybackControls(
        height: 480,
        audioLanguage: 'fra',
        subtitleLanguage: 'fra',
        offsetMs: 50,
      );
      final PlaybackControls got = asked.withSelection(
        const SelectedTracks(
          audioTrack: 4,
          subtitleTrack: SubtitleSelection.embedded(6),
        ),
      );
      expect(got.audioTrack, 4);
      expect(got.subtitle, const SubtitleSelection.embedded(6));
      expect(got.height, 480);
      expect(got.offsetMs, 50);
      expect(
        got.audioLanguage,
        'fra',
        reason:
            'the language is what travels to the next episode, so resolving it '
            'here must not consume it',
      );
      expect(got.subtitleLanguage, 'fra');
    });

    test('a request the server could not honour leaves the carry standing', () {
      const PlaybackControls asked = PlaybackControls(subtitleLanguage: 'fra');
      final PlaybackControls got = asked.withSelection(const SelectedTracks());
      expect(got.subtitle, isNull);
      expect(
        got.subtitleLanguage,
        'fra',
        reason:
            'this episode has no French, but the one after it might, and the '
            'viewer never withdrew the request',
      );
    });

    test('equality covers every field', () {
      const PlaybackControls base = PlaybackControls();
      expect(base, const PlaybackControls());
      expect(base.hashCode, const PlaybackControls().hashCode);
      for (final PlaybackControls other in <PlaybackControls>[
        const PlaybackControls(height: 720),
        const PlaybackControls(burn: true),
        const PlaybackControls(downmix: true),
        const PlaybackControls(audioTrack: 1),
        const PlaybackControls(audioLanguage: 'eng'),
        const PlaybackControls(subtitle: SubtitleSelection.embedded(1)),
        const PlaybackControls(subtitleLanguage: 'eng'),
        const PlaybackControls(subtitleOff: true),
        const PlaybackControls(offsetMs: 5),
      ]) {
        expect(base, isNot(other));
        expect(other.isDefault, isFalse);
      }
    });
  });
}
