import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/player/state/player_tracks.dart';

const VersionDetail _version = VersionDetail(
  id: 'v1',
  title: TitleRef(type: TitleKind.movie, id: 't1'),
  libraryId: 'lib',
  quality: Quality.hd,
  container: 'mkv',
  subtitles: <SubtitleTrack>[
    SubtitleTrack(index: 3, format: SubtitleFormat.srt),
    SubtitleTrack(index: 4, format: SubtitleFormat.pgs),
    SubtitleTrack(index: 5, format: SubtitleFormat.dvb),
  ],
);

PlayerTracks _tracks(SelectedTracks selected) =>
    playerTracks(selected, _version);

void main() {
  test('no answer loads nothing beyond the stream', () {
    expect(playerTracks(null, _version), const PlayerTracks());
  });

  test('a subtitle in the stream is shown from the stream', () {
    expect(
      _tracks(
        const SelectedTracks(
          subtitleTrack: SubtitleSelection.embedded(3),
          subtitleDelivery: 'hls_vtt',
        ),
      ),
      const PlayerTracks(rendition: true),
    );
  });

  test('an embedded text track is selected in the file', () {
    expect(
      _tracks(
        const SelectedTracks(
          subtitleTrack: SubtitleSelection.embedded(3),
          subtitleDelivery: 'embedded',
        ),
      ),
      const PlayerTracks(subtitleIndex: 3),
    );
  });

  test('an embedded image track is selected and drawn by the player', () {
    for (final int index in <int>[4, 5]) {
      expect(
        _tracks(
          SelectedTracks(
            subtitleTrack: SubtitleSelection.embedded(index),
            subtitleDelivery: 'embedded',
          ),
        ),
        PlayerTracks(subtitleIndex: index, subtitleImage: true),
      );
    }
  });

  test('a subtitle from a file is loaded from its address', () {
    expect(
      _tracks(
        const SelectedTracks(
          subtitleTrack: SubtitleSelection.file('f1'),
          subtitleDelivery: 'external',
          subtitleUrl: '/stream/t1/subtitles.vtt',
        ),
      ),
      const PlayerTracks(subtitleUrl: '/stream/t1/subtitles.vtt'),
    );
  });

  test('a burned subtitle loads nothing in the player', () {
    expect(
      _tracks(
        const SelectedTracks(
          subtitleTrack: SubtitleSelection.embedded(4),
          subtitleDelivery: 'burned',
        ),
      ),
      const PlayerTracks(),
    );
  });

  test('an audio track is selected only when the player picks it', () {
    expect(
      _tracks(const SelectedTracks(audioTrack: 2, audioDelivery: 'embedded')),
      const PlayerTracks(audioIndex: 2),
    );
    expect(
      _tracks(const SelectedTracks(audioTrack: 2, audioDelivery: 'stream')),
      const PlayerTracks(),
    );
  });

  test('a track the version does not list is selected but not drawn', () {
    expect(
      playerTracks(
        const SelectedTracks(
          subtitleTrack: SubtitleSelection.embedded(9),
          subtitleDelivery: 'embedded',
        ),
        null,
      ),
      const PlayerTracks(subtitleIndex: 9),
    );
  });
}
