import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/discovery/continue_feed.dart';
import 'package:shadowmask/view/catalog_card.dart';

Map<String, dynamic> _card(String title) => <String, dynamic>{
  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
  'display_title': title,
  'duration_ms': 600000,
  'progress_percent': 16,
};

void main() {
  // The two lists carry different shapes: an in_progress entry nests the
  // version under progress, while a now_playing entry is a session and carries
  // it at the top level. Reading only the nested one left the card that is
  // actually playing with no way to dismiss it.
  test('both continue shapes yield a dismissable card', () {
    final ContinueFeed feed = ContinueFeed.fromJson(<String, dynamic>{
      'now_playing': <dynamic>[
        <String, dynamic>{
          'session_id': 's1',
          'user_id': 'u1',
          'version_id': 'v-playing',
          'position_ms': 96000,
          'card': _card('Playing now'),
        },
      ],
      'in_progress': <dynamic>[
        <String, dynamic>{
          'progress': <String, dynamic>{'version_id': 'v-paused'},
          'card': _card('Left off'),
        },
      ],
    });

    expect(feed.continueWatching.length, 2);
    expect(
      feed.continueWatching.map((CatalogCard c) => c.dismissVersionId).toList(),
      <String?>['v-playing', 'v-paused'],
    );
  });

  test('an entry with no version anywhere is simply not dismissable', () {
    final ContinueFeed feed = ContinueFeed.fromJson(<String, dynamic>{
      'now_playing': <dynamic>[
        <String, dynamic>{'card': _card('No version')},
      ],
    });

    expect(feed.continueWatching.single.dismissVersionId, isNull);
  });

  test('resume progress is keyed by the version it would clear', () {
    final ContinueFeed feed = ContinueFeed.fromJson(<String, dynamic>{
      'now_playing': <dynamic>[
        <String, dynamic>{'version_id': 'v-playing', 'card': _card('Playing')},
      ],
    });

    expect(feed.resumeProgress, <String, int>{'v-playing': 16});
  });

  test('an entry without a card is dropped rather than half built', () {
    final ContinueFeed feed = ContinueFeed.fromJson(<String, dynamic>{
      'now_playing': <dynamic>[
        <String, dynamic>{'session_id': 's1', 'version_id': 'v1'},
      ],
    });

    expect(feed.continueWatching, isEmpty);
    expect(feed.isEmpty, isTrue);
  });
}
