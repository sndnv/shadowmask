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

  test('SelectedTracks reads how each track reaches the player', () {
    final SelectedTracks st = SelectedTracks.fromJson(<String, dynamic>{
      'audio_track': 2,
      'audio_delivery': 'embedded',
      'subtitle_track': <String, dynamic>{'type': 'file', 'id': 'f1'},
      'subtitle_delivery': 'external',
      'subtitle_url': '/stream/t1/subtitles.vtt',
    });
    expect(st.audioDelivery, 'embedded');
    expect(st.subtitleDelivery, 'external');
    expect(st.subtitleUrl, '/stream/t1/subtitles.vtt');
  });
}
