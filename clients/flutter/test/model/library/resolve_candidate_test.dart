import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/library/resolve_candidate.dart';

void main() {
  test('ResolveCandidate parses an existing-title target', () {
    final ResolveCandidate c = ResolveCandidate.fromJson(<String, dynamic>{
      'source': 'catalog',
      'target': <String, dynamic>{
        'kind': 'existing',
        'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      },
      'title': 'The Matrix',
      'year': 1999,
      'kind': 'movie',
    });

    expect(c.source, 'catalog');
    expect(c.year, 1999);
    expect(c.target.kind, 'existing');
    expect(c.target.title?.type, TitleKind.movie);
    expect(c.target.title?.id, 'm1');
    expect(c.target.source, isNull);
  });

  test('ResolveCandidate parses a provider target', () {
    final ResolveCandidate c = ResolveCandidate.fromJson(<String, dynamic>{
      'source': 'tmdb',
      'target': <String, dynamic>{
        'kind': 'provider',
        'source': 'tmdb',
        'value': 'movie/603',
      },
      'title': 'The Matrix',
      'kind': 'movie',
    });

    expect(c.target.kind, 'provider');
    expect(c.target.source, 'tmdb');
    expect(c.target.value, 'movie/603');
    expect(c.target.title, isNull);
  });
}
