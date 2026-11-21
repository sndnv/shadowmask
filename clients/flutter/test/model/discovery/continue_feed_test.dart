import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/discovery/continue_feed.dart';
import 'package:shadowmask/nav/routes.dart';
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

  test('the queue\'s next title leads Up next and opens its page queued', () {
    final ContinueFeed feed = ContinueFeed.fromJson(<String, dynamic>{
      'next_episodes': <dynamic>[
        <String, dynamic>{
          'id': 'e2',
          'season_id': 'se1',
          'number': 2,
          'title': 'Second',
        },
      ],
      'queue_next': <String, dynamic>{
        'id': 'q7',
        'card': <String, dynamic>{
          'type': 'movie',
          'id': 'm5',
          'title': 'Fifth',
        },
      },
    });

    expect(feed.queueNext?.id, 'q7');
    expect(feed.upNext.map((CatalogCard c) => c.ref.id), <String>['m5', 'e2']);
    expect(feed.upNext.first.route, queuedRoute(movieRoute('m5'), 'q7'));
    expect(feed.upNext.last.route, startsWith('/episode'));
  });

  test('without a queue Up next is the series and movies alone', () {
    final ContinueFeed feed = ContinueFeed.fromJson(<String, dynamic>{
      'queue_next': null,
      'next_movies': <dynamic>[
        <String, dynamic>{'id': 'm2', 'title': 'Two'},
      ],
    });

    expect(feed.queueNext, isNull);
    expect(feed.upNext.map((CatalogCard c) => c.ref.id), <String>['m2']);
  });

  test('a queued movie that is also next in its collection shows once', () {
    final ContinueFeed feed = ContinueFeed.fromJson(<String, dynamic>{
      'next_movies': <dynamic>[
        <String, dynamic>{'id': 'm5', 'title': 'Fifth'},
      ],
      'queue_next': <String, dynamic>{
        'id': 'q7',
        'card': <String, dynamic>{
          'type': 'movie',
          'id': 'm5',
          'title': 'Fifth',
        },
      },
    });

    expect(feed.upNext.map((CatalogCard c) => c.route), <String>[
      queuedRoute(movieRoute('m5'), 'q7'),
    ]);
  });

  test('a queued episode stands in for its series\' next episode', () {
    final ContinueFeed feed = ContinueFeed.fromJson(<String, dynamic>{
      'next_episodes': <dynamic>[
        <String, dynamic>{
          'id': 'e2',
          'season_id': 'se1',
          'number': 2,
          'title': 'Second',
          'series_id': 's1',
        },
        <String, dynamic>{
          'id': 'e9',
          'season_id': 'se4',
          'number': 1,
          'title': 'Other',
          'series_id': 's2',
        },
      ],
      'queue_next': <String, dynamic>{
        'id': 'q7',
        'card': <String, dynamic>{
          'type': 'episode',
          'id': 'e5',
          'season_id': 'se1',
          'number': 5,
          'title': 'Fifth',
          'series_id': 's1',
        },
      },
    });

    expect(feed.upNext.map((CatalogCard c) => c.ref.id), <String>['e5', 'e9']);
    final Uri queued = Uri.parse(feed.upNext.first.route);
    expect(queued.path, '/episode');
    expect(queued.queryParameters['queue'], 'q7');
  });

  test('a movie next in two collections shows once', () {
    final ContinueFeed feed = ContinueFeed.fromJson(<String, dynamic>{
      'next_movies': <dynamic>[
        <String, dynamic>{'id': 'm1', 'title': 'One'},
        <String, dynamic>{'id': 'm2', 'title': 'Two'},
        <String, dynamic>{'id': 'm1', 'title': 'One'},
      ],
    });

    expect(feed.upNext.map((CatalogCard c) => c.ref.id), <String>['m1', 'm2']);
  });

  test('episodes that name no series are kept apart', () {
    final ContinueFeed feed = ContinueFeed.fromJson(<String, dynamic>{
      'next_episodes': <dynamic>[
        <String, dynamic>{
          'id': 'e1',
          'season_id': 'se1',
          'number': 1,
          'title': 'One',
        },
        <String, dynamic>{
          'id': 'e2',
          'season_id': 'se2',
          'number': 1,
          'title': 'Two',
        },
      ],
    });

    expect(feed.upNext.map((CatalogCard c) => c.ref.id), <String>['e1', 'e2']);
  });

  test('once per title keeps the first card and tells kinds apart', () {
    final ContinueFeed feed = ContinueFeed.fromJson(<String, dynamic>{
      'now_playing': <dynamic>[
        <String, dynamic>{'version_id': 'v1', 'card': _card('Playing')},
      ],
      'in_progress': <dynamic>[
        <String, dynamic>{
          'progress': <String, dynamic>{'version_id': 'v2'},
          'card': _card('Left off'),
        },
        <String, dynamic>{
          'progress': <String, dynamic>{'version_id': 'v3'},
          'card': <String, dynamic>{
            ..._card('Other'),
            'title': <String, dynamic>{'type': 'episode', 'id': 'm1'},
          },
        },
      ],
    });

    expect(feed.resumeProgress.keys, <String>['v1', 'v2', 'v3']);
    expect(
      oncePerTitle(
        feed.continueWatching,
      ).map((CatalogCard c) => c.dismissVersionId),
      <String?>['v1', 'v3'],
    );
  });
}
