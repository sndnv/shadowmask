import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/movie.dart';
import 'package:shadowmask/view/page.dart';

void main() {
  test('paged parses meta and computes bounds', () {
    final Paged<Movie> p = Paged<Movie>.fromJson(<String, dynamic>{
      'items': <dynamic>[
        <String, dynamic>{'id': 'm1', 'title': 'Alpha'},
      ],
      'total': 3,
      'offset': 0,
      'limit': 1,
    }, Movie.fromJson);
    expect(p.items.single.title, 'Alpha');
    expect(p.hasNext, isTrue);
    expect(p.hasPrev, isFalse);
  });
}
