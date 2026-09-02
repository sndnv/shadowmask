import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user_library/watched_rollup.dart';

void main() {
  test('WatchedRollup parses the target ref and counts', () {
    final WatchedRollup r = WatchedRollup.fromJson(<String, dynamic>{
      'target': <String, dynamic>{'type': 'series', 'id': 's1'},
      'watched': true,
      'completed': false,
      'watched_episodes': 3,
      'total_episodes': 10,
    });

    expect(r.target.type, TitleKind.series);
    expect(r.target.id, 's1');
    expect(r.watched, isTrue);
    expect(r.watchedEpisodes, 3);
    expect(r.totalEpisodes, 10);
  });

  test('WatchedRollup defaults counts to zero', () {
    final WatchedRollup r = WatchedRollup.fromJson(<String, dynamic>{
      'target': <String, dynamic>{'type': 'season', 'id': 'se1'},
      'watched': false,
      'completed': false,
    });

    expect(r.watchedEpisodes, 0);
    expect(r.totalEpisodes, 0);
  });
}
