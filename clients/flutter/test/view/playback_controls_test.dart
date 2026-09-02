import 'package:flutter_test/flutter_test.dart';
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

    test('the default carries no query and is default', () {
      expect(const PlaybackControls().toQuery(), isEmpty);
      expect(const PlaybackControls().isDefault, isTrue);
    });
  });
}
