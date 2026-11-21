import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/nav/route_args.dart';
import 'package:shadowmask/nav/routes.dart';

void main() {
  test('a watch route carries a start position after the controls', () {
    expect(
      watchRoute('v2', controls: <String, String?>{'alang': 'en'}, atMs: 61000),
      '/watch?version=v2&alang=en&at=61000',
    );
    expect(watchRoute('v2'), '/watch?version=v2');
  });

  test('a start position is read only when it is a whole number of zero or '
      'more', () {
    expect(startAtArg(<String, String>{'at': '61000'}), 61000);
    expect(startAtArg(<String, String>{'at': '0'}), 0);
    expect(startAtArg(<String, String>{'at': '-5'}), isNull);
    expect(startAtArg(<String, String>{'at': 'soon'}), isNull);
    expect(startAtArg(<String, String>{}), isNull);
  });

  test('a queued route keeps its page and adds the queue entry', () {
    expect(
      queuedRoute(movieRoute('m1'), 'q3'),
      '/title?type=movie&id=m1&queue=q3',
    );
    expect(
      queuedRoute(episodeRoute('e1', series: 's1', season: 'se1'), 'q4'),
      '/episode?id=e1&season=se1&series=s1&queue=q4',
    );
  });

  test('withQuery omits null and empty values', () {
    expect(
      withQuery('/movies', <String, String?>{
        'sort': 'title',
        'genres': null,
        'offset': '',
      }),
      '/movies?sort=title',
    );
  });
}
