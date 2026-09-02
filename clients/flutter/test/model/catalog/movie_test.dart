import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/movie.dart';

void main() {
  test('movie parses snake_case fields and resolves artwork urls', () {
    final Movie m = Movie.fromJson(<String, dynamic>{
      'id': 'm1',
      'title': 'Alpha',
      'year': 2020,
      'runtime_minutes': 100,
      'content_rating': <String, dynamic>{'system': 'MPAA', 'code': 'PG'},
      'added_at': '1999-01-01T00:00:00Z',
      'updated_at': '1999-01-02T00:00:00Z',
      'artwork': <String, dynamic>{
        'posters': <dynamic>[
          <String, dynamic>{
            'base': '/images/p1',
            'widths': <int>[180, 480, 960],
          },
        ],
      },
    });
    expect(m.runtimeMinutes, 100);
    expect(m.contentRating!.code, 'PG');
    expect(m.artwork!.posterUrl('http://h', 480), 'http://h/images/p1/480');
    expect(m.artwork!.posterUrl('http://h', 200), 'http://h/images/p1/480');
    expect(m.artwork!.posterUrl('http://h', 1000), 'http://h/images/p1/960');
  });
}
