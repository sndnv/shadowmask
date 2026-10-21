import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/player/mpv_tracks.dart';

const List<MpvTrack> _listed = <MpvTrack>[
  MpvTrack(id: 1, type: 'video', ffIndex: 0, external: false),
  MpvTrack(id: 1, type: 'audio', ffIndex: 1, external: false),
  MpvTrack(id: 2, type: 'audio', ffIndex: 2, external: false),
  MpvTrack(id: 1, type: 'sub', ffIndex: 3, external: false),
  MpvTrack(id: 2, type: 'sub', ffIndex: 4, external: false),
  MpvTrack(id: 3, type: 'sub', ffIndex: 2, external: true),
];

void main() {
  test('a stream of the file is found by its index and kind', () {
    expect(mpvTrackId(_listed, 'audio', 2), 2);
    expect(mpvTrackId(_listed, 'sub', 4), 2);
  });

  test('a stream of another kind or a loaded file never matches', () {
    expect(mpvTrackId(_listed, 'sub', 1), isNull);
    expect(mpvTrackId(_listed, 'sub', 2), isNull);
  });

  test('a stream the file does not have is not found', () {
    expect(mpvTrackId(_listed, 'audio', 7), isNull);
  });

  test('a listed track is read from mpv\'s properties', () {
    final MpvTrack? track = mpvTrackOf(
      id: '2',
      type: 'sub',
      ffIndex: '4',
      external: 'no',
    );

    expect(track?.id, 2);
    expect(track?.type, 'sub');
    expect(track?.ffIndex, 4);
    expect(track?.external, isFalse);
    expect(
      mpvTrackOf(id: '3', type: 'sub', ffIndex: '', external: 'yes')?.external,
      isTrue,
    );
  });

  test('a track mpv could not describe is left out', () {
    expect(
      mpvTrackOf(id: '', type: 'sub', ffIndex: '4', external: 'no'),
      isNull,
    );
    expect(mpvTrackOf(id: '2', type: '', ffIndex: '4', external: 'no'), isNull);
  });
}
