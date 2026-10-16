import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/player/playback_log.dart';

void main() {
  test('each line carries the time it was noted', () {
    final PlaybackLog log = PlaybackLog(
      now: () => DateTime(2026, 10, 15, 9, 49, 52),
    );

    log.add('ended at 231.4/596.0 s, buffered 0.0-532.0');

    expect(log.lines, <String>[
      '09:49:52 ended at 231.4/596.0 s, buffered 0.0-532.0',
    ]);
  });

  test('the log keeps the newest lines up to its limit', () {
    final PlaybackLog log = PlaybackLog(now: () => DateTime(2026));

    for (int i = 0; i <= kPlaybackLogLimit; i++) {
      log.add('line $i');
    }

    expect(log.lines, hasLength(kPlaybackLogLimit));
    expect(log.lines.first, '00:00:00 line 1');
    expect(log.lines.last, '00:00:00 line $kPlaybackLogLimit');
  });

  test('the overlay takes the last few lines', () {
    final PlaybackLog log = PlaybackLog(now: () => DateTime(2026));

    for (int i = 0; i < 10; i++) {
      log.add('line $i');
    }

    expect(log.recent(), hasLength(kOverlayLogLines));
    expect(log.recent().first, '00:00:00 line 2');
    expect(log.recent().last, '00:00:00 line 9');
  });

  test('an event names the position, duration, buffer and a hidden page', () {
    expect(
      playbackEventLine(
        'ended',
        positionS: 231.4,
        durationS: 596,
        buffered: <(double, double)>[(0, 120), (180, 532)],
        hidden: true,
      ),
      'ended at 231.4/596.0 s, buffered 0.0-120.0 180.0-532.0, hidden',
    );
  });

  test('an event before the duration is known still reads', () {
    expect(
      playbackEventLine(
        'waiting',
        positionS: 0,
        durationS: double.nan,
        buffered: const <(double, double)>[],
        hidden: false,
      ),
      'waiting at 0.0/? s, buffered none',
    );
  });

  test('a long line is cut for the overlay and a short one is kept', () {
    final String long = 'x' * (kOverlayLineLength + 50);

    expect(clipLine(long), '${'x' * kOverlayLineLength}…');
    expect(clipLine('mode: direct'), 'mode: direct');
  });

  test('a cut keeps a character that spans two code units whole', () {
    final String prefix = 'x' * (kOverlayLineLength - 1);

    expect(clipLine('$prefix😀 tail'), '$prefix😀…');
  });
}
