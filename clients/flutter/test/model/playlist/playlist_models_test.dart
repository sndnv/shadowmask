import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/playlist/playlist.dart';
import 'package:shadowmask/model/playlist/playlist_detail.dart';
import 'package:shadowmask/model/playlist/queue_entry.dart';
import 'package:shadowmask/nav/routes.dart';

Map<String, dynamic> _episodeCard() => <String, dynamic>{
  'type': 'episode',
  'id': 'e1',
  'season_id': 'se1',
  'series_id': 's1',
  'series_title': 'The Show',
  'season_number': 1,
  'number': 2,
  'title': 'Second',
};

void main() {
  test('a listed playlist carries its visible count; a new one has none', () {
    final Playlist listed = Playlist.fromJson(<String, dynamic>{
      'id': 'pl1',
      'name': 'Weekend',
      'item_count': 3,
      'created_at': '2026-11-06T10:00:00Z',
      'updated_at': '2026-11-06T11:00:00Z',
    });
    final Playlist made = Playlist.fromJson(<String, dynamic>{
      'id': 'pl2',
      'name': 'New',
      'created_at': '2026-11-06T10:00:00Z',
      'updated_at': '2026-11-06T10:00:00Z',
    });

    expect((listed.name, listed.itemCount), ('Weekend', 3));
    expect(made.itemCount, 0);
  });

  test('a playlist reads its items in order with series-led episode cards', () {
    final PlaylistDetail detail = PlaylistDetail.fromJson(<String, dynamic>{
      'id': 'pl1',
      'name': 'Weekend',
      'created_at': '2026-11-06T10:00:00Z',
      'updated_at': '2026-11-06T10:00:00Z',
      'items': <dynamic>[
        <String, dynamic>{
          'position': 0,
          'added_at': '2026-11-06T10:00:00Z',
          'card': <String, dynamic>{
            'type': 'movie',
            'id': 'm1',
            'title': 'One',
          },
        },
        <String, dynamic>{
          'position': 2,
          'added_at': '2026-11-06T10:00:00Z',
          'card': _episodeCard(),
        },
      ],
    });

    expect(detail.playlist.id, 'pl1');
    expect(detail.items.map((i) => i.position), <int>[0, 2]);
    expect(detail.items.first.card.ref.type, TitleKind.movie);
    expect(detail.items.last.card.title, 'The Show');
    expect(detail.items.last.card.caption, 'Second');
  });

  test('a playlist without items reads as empty', () {
    final PlaylistDetail detail = PlaylistDetail.fromJson(<String, dynamic>{
      'id': 'pl1',
      'name': 'Empty',
      'created_at': '2026-11-06T10:00:00Z',
      'updated_at': '2026-11-06T10:00:00Z',
    });

    expect(detail.items, isEmpty);
  });

  test('a queue entry plays from itself in queue mode', () {
    final QueueEntry entry = QueueEntry.fromJson(<String, dynamic>{
      'id': '12',
      'card': _episodeCard(),
    });

    expect(entry.id, '12');
    expect(entry.card.ref, const TitleRef(type: TitleKind.episode, id: 'e1'));
    expect(entry.card.route, queuePlayRoute('12'));
    expect(entry.card.queueEntryId, '12');
  });
}
