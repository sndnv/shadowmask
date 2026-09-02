import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/catalog/movie_detail.dart';

void main() {
  test('movie detail parses the enrichment arrays', () {
    final MovieDetail d = MovieDetail.fromJson(<String, dynamic>{
      'id': 'm1',
      'title': 'Alpha',
      'genres': <dynamic>[
        <String, dynamic>{'id': 'g1', 'name': 'Action'},
      ],
      'credits': <dynamic>[
        <String, dynamic>{
          'person': <String, dynamic>{'id': 'p1', 'name': 'Ada'},
          'role': 'actor',
          'character': 'Hero',
          'order': 0,
        },
      ],
    });
    expect(d.genres.single.name, 'Action');
    expect(d.credits.single.role, CreditRole.actor);
    expect(d.credits.single.person.name, 'Ada');
  });
}
