import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/view/sound_settings.dart';

void main() {
  group('DialogueLift', () {
    test('reads from its decibels, anything else is none', () {
      expect(DialogueLift.fromDb(0), DialogueLift.none);
      expect(DialogueLift.fromDb(3), DialogueLift.db3);
      expect(DialogueLift.fromDb(6), DialogueLift.db6);
      expect(DialogueLift.fromDb(9), DialogueLift.db9);
      expect(DialogueLift.fromDb(4), DialogueLift.none);
      expect(DialogueLift.fromDb(null), DialogueLift.none);
    });

    test('lowers everything else by its decibels', () {
      expect(DialogueLift.none.othersGain, 1.0);
      expect(DialogueLift.db3.othersGain, closeTo(0.707946, 0.000001));
      expect(DialogueLift.db6.othersGain, closeTo(0.501187, 0.000001));
      expect(DialogueLift.db9.othersGain, closeTo(0.354813, 0.000001));
    });
  });

  group('NightMode', () {
    test('reads from the wire, anything else is off', () {
      expect(NightMode.fromWire('light'), NightMode.light);
      expect(NightMode.fromWire('strong'), NightMode.strong);
      expect(NightMode.fromWire('off'), NightMode.off);
      expect(NightMode.fromWire('loud'), NightMode.off);
      expect(NightMode.fromWire(null), NightMode.off);
    });
  });

  group('mpvSound', () {
    test('normalizes and leaves the centre alone by default', () {
      expect(
        mpvSound(normalize: true, dialogue: DialogueLift.none),
        <String, String>{
          'audio-normalize-downmix': 'yes',
          'audio-swresample-o': '',
        },
      );
    });

    test('a lift raises the centre in mpv\'s own mix', () {
      for (final (DialogueLift lift, String centre) in <(DialogueLift, String)>[
        (DialogueLift.db3, '0.998815'),
        (DialogueLift.db6, '1.410864'),
        (DialogueLift.db9, '1.992898'),
      ]) {
        expect(mpvSound(normalize: false, dialogue: lift), <String, String>{
          'audio-normalize-downmix': 'no',
          'audio-swresample-o': 'center_mix_level=$centre',
        }, reason: '$lift');
      }
    });
  });

  group('soundRows', () {
    test('no sound shows nothing', () {
      expect(
        soundRows(
          source: null,
          sent: null,
          output: null,
          mixesItself: true,
          dialogue: DialogueLift.db6,
        ),
        (downmix: false, normalize: false, dialogue: false, night: false),
      );
    });

    test('mono shows night mode only', () {
      expect(
        soundRows(
          source: 1,
          sent: 1,
          output: 2,
          mixesItself: true,
          dialogue: DialogueLift.db6,
        ),
        (downmix: false, normalize: false, dialogue: false, night: true),
      );
    });

    test('stereo shows dialogue and night mode', () {
      for (final bool mixesItself in <bool>[true, false]) {
        expect(
          soundRows(
            source: 2,
            sent: 2,
            output: 2,
            mixesItself: mixesItself,
            dialogue: DialogueLift.none,
          ),
          (downmix: false, normalize: false, dialogue: true, night: true),
          reason: 'mixes itself: $mixesItself',
        );
      }
    });

    test('stereo with a lift shows normalize, since the server folds the '
        'lifted centre back to stereo', () {
      for (final bool mixesItself in <bool>[true, false]) {
        expect(
          soundRows(
            source: 2,
            sent: 2,
            output: 2,
            mixesItself: mixesItself,
            dialogue: DialogueLift.db3,
          ),
          (downmix: false, normalize: true, dialogue: true, night: true),
          reason: 'mixes itself: $mixesItself',
        );
      }
    });

    test('surround the server folds shows everything', () {
      for (final bool mixesItself in <bool>[true, false]) {
        expect(
          soundRows(
            source: 6,
            sent: 2,
            output: 2,
            mixesItself: mixesItself,
            dialogue: DialogueLift.none,
          ),
          (downmix: true, normalize: true, dialogue: true, night: true),
          reason: 'mixes itself: $mixesItself',
        );
      }
    });

    test('surround a player folds itself shows everything', () {
      expect(
        soundRows(
          source: 6,
          sent: 6,
          output: 2,
          mixesItself: true,
          dialogue: DialogueLift.none,
        ),
        (downmix: true, normalize: true, dialogue: true, night: true),
      );
      expect(
        soundRows(
          source: 6,
          sent: 6,
          output: null,
          mixesItself: true,
          dialogue: DialogueLift.none,
        ),
        (downmix: true, normalize: true, dialogue: true, night: true),
        reason: 'before the output is known the player is taken to fold',
      );
    });

    test('surround on surround speakers hides normalize and dialogue', () {
      for (final DialogueLift lift in <DialogueLift>[
        DialogueLift.none,
        DialogueLift.db6,
      ]) {
        expect(
          soundRows(
            source: 6,
            sent: 6,
            output: 6,
            mixesItself: true,
            dialogue: lift,
          ),
          (downmix: true, normalize: false, dialogue: false, night: true),
          reason: '$lift',
        );
      }
    });

    test('surround sent to a player that leaves the mix to others hides '
        'normalize only', () {
      for (final DialogueLift lift in <DialogueLift>[
        DialogueLift.none,
        DialogueLift.db6,
      ]) {
        expect(
          soundRows(
            source: 6,
            sent: 6,
            output: null,
            mixesItself: false,
            dialogue: lift,
          ),
          (downmix: true, normalize: false, dialogue: true, night: true),
          reason: '$lift',
        );
      }
    });

    test('a server fold to more than two channels shows normalize', () {
      expect(
        soundRows(
          source: 8,
          sent: 6,
          output: 6,
          mixesItself: false,
          dialogue: DialogueLift.none,
        ),
        (downmix: true, normalize: true, dialogue: true, night: true),
      );
    });
  });

  group('reaching the server', () {
    test('normalize matters to the server only when it folds', () {
      expect(
        normalizeReachesServer(source: 6, sent: 2, dialogue: DialogueLift.none),
        isTrue,
      );
      expect(
        normalizeReachesServer(source: 6, sent: 6, dialogue: DialogueLift.none),
        isFalse,
      );
      expect(
        normalizeReachesServer(
          source: 2,
          sent: null,
          dialogue: DialogueLift.none,
        ),
        isFalse,
      );
      expect(
        normalizeReachesServer(source: 2, sent: 2, dialogue: DialogueLift.db9),
        isTrue,
      );
      expect(
        normalizeReachesServer(source: 1, sent: 1, dialogue: DialogueLift.db9),
        isFalse,
      );
      expect(
        normalizeReachesServer(source: 6, sent: 6, dialogue: DialogueLift.db9),
        isFalse,
      );
    });

    test(
      'a lift reaches the server unless the player lifts in its own mix',
      () {
        expect(
          liftReachesServer(source: 2, sent: 2, mixesItself: true),
          isTrue,
        );
        expect(
          liftReachesServer(source: 6, sent: 2, mixesItself: true),
          isTrue,
        );
        expect(
          liftReachesServer(source: 6, sent: 6, mixesItself: true),
          isFalse,
        );
        expect(
          liftReachesServer(source: 6, sent: 6, mixesItself: false),
          isTrue,
        );
        expect(
          liftReachesServer(source: 1, sent: 1, mixesItself: false),
          isFalse,
        );
      },
    );
  });
}
