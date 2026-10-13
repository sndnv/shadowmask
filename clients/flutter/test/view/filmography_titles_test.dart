import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/person_profile.dart';
import 'package:shadowmask/view/filmography_titles.dart';

FilmographyEntry _credit(String kind, String id, String role) =>
    FilmographyEntry.fromJson(<String, dynamic>{
      'title_id': id,
      'kind': kind,
      'display_title': 'Title $id',
      'role': role,
    });

void main() {
  test('the credits on one title come together, in the order first seen', () {
    final List<List<FilmographyEntry>> titles =
        filmographyByTitle(<FilmographyEntry>[
          _credit('movie', 'm2', 'actor'),
          _credit('movie', 'm1', 'director'),
          _credit('movie', 'm2', 'writer'),
          _credit('series', 'm2', 'actor'),
        ]);

    expect(
      titles
          .map(
            (List<FilmographyEntry> credits) => credits
                .map((FilmographyEntry c) => '${c.titleId}:${c.role.name}')
                .join(' '),
          )
          .toList(),
      <String>['m2:actor m2:writer', 'm1:director', 'm2:actor'],
      reason: 'a movie and a series sharing an id are two titles',
    );
  });

  test('no credits make no titles', () {
    expect(filmographyByTitle(<FilmographyEntry>[]), isEmpty);
  });
}
