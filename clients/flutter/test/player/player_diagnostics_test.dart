import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/player/player_diagnostics.dart';

void main() {
  // A stream token is a JWT, so one log line pushed everything else out of the
  // overlay and left nothing readable.
  test('a stream token is cut out of a log line', () {
    const String line =
        'e ffmpeg/demuxer: hls: Error when loading first segment '
        'http://localhost:8080/stream/eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ4In0.'
        'abc-def_123/v0/seg_00000.ts';

    final String cut = redactStreamTokens(line);

    expect(cut.contains('eyJ'), isFalse);
    expect(cut, contains('/stream/…/v0/seg_00000.ts'));
  });

  test('more than one token in a line is cut', () {
    expect(
      redactStreamTokens('/stream/aaa/v0/a.ts and /stream/bbb/v0/b.ts'),
      '/stream/…/v0/a.ts and /stream/…/v0/b.ts',
    );
  });

  test('a line with nothing to cut is left alone', () {
    const String line = 'w cplayer: Invalid video timestamp: 4.2 -> 4.2';

    expect(redactStreamTokens(line), line);
  });
}
