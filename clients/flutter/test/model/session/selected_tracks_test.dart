import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';

void main() {
  test('SelectedTracks tolerates a null subtitle', () {
    final SelectedTracks st = SelectedTracks.fromJson(<String, dynamic>{
      'audio_track': 2,
      'subtitle_track': null,
    });
    expect(st.audioTrack, 2);
    expect(st.subtitleTrack, isNull);
  });
}
