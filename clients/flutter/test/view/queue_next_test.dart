import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/playlist/queue_entry.dart';
import 'package:shadowmask/view/queue_next.dart';
import 'package:shadowmask/view/title_link.dart';

QueueEntry _movie(String entry, String id, String title) =>
    QueueEntry.fromJson(<String, dynamic>{
      'id': entry,
      'card': <String, dynamic>{'type': 'movie', 'id': id, 'title': title},
    });

QueueEntry _episode(String entry) => QueueEntry.fromJson(<String, dynamic>{
  'id': entry,
  'card': <String, dynamic>{
    'type': 'episode',
    'id': 'e2',
    'season_id': 'se1',
    'series_title': 'The Show',
    'season_number': 1,
    'number': 2,
    'title': 'Second',
  },
});

void main() {
  final List<QueueEntry> queue = <QueueEntry>[
    _movie('1', 'm1', 'One'),
    _episode('2'),
    _movie('3', 'm1', 'One'),
  ];

  test('next is the entry after the one playing', () {
    expect(queueNext(queue, '1')?.id, '2');
    expect(queueNext(queue, '2')?.id, '3');
  });

  test('the last entry has no next', () {
    expect(queueNext(queue, '3'), isNull);
  });

  test('an entry already gone from the queue hands over to the first', () {
    expect(queueNext(queue, '9')?.id, '1');
    expect(queueNext(const <QueueEntry>[], '9'), isNull);
  });

  test('an episode is labelled by its series, code and title; a movie by '
      'its title', () {
    expect(queueLabel(queue[1].card), 'The Show · S01E02 · Second');
    expect(queueLabel(queue[0].card), 'One');
  });

  test('a link carries the kind, the title and the queue entry', () {
    final TitleLink link = queueLink(queue[1]);

    expect(link.kind, TitleKind.episode);
    expect(link.id, 'e2');
    expect(link.queueEntryId, '2');
    expect(link.label, 'The Show · S01E02 · Second');
  });
}
