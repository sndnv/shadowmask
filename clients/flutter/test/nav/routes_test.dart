import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/nav/routes.dart';

void main() {
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
