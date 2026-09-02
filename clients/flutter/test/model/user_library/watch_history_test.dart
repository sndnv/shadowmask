import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user_library/watch_history.dart';

void main() {
  test('WatchHistory parses the title ref and playback fields', () {
    final WatchHistory h = WatchHistory.fromJson(<String, dynamic>{
      'user_id': 'u1',
      'title': <String, dynamic>{'type': 'episode', 'id': 'e1'},
      'watched': true,
      'play_count': 2,
      'last_watched_at': '2026-08-01T00:00:00Z',
      'completed': true,
    });

    expect(h.title.type, TitleKind.episode);
    expect(h.playCount, 2);
    expect(h.completed, isTrue);
    expect(h.lastWatchedAt, '2026-08-01T00:00:00Z');
  });

  test('WatchHistory tolerates a missing last watched timestamp', () {
    final WatchHistory h = WatchHistory.fromJson(<String, dynamic>{
      'user_id': 'u1',
      'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      'watched': false,
      'play_count': 0,
      'completed': false,
    });

    expect(h.lastWatchedAt, isNull);
    expect(h.title.type, TitleKind.movie);
  });
}
