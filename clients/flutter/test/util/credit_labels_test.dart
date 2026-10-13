import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/person_profile.dart';
import 'package:shadowmask/util/credit_labels.dart';

FilmographyEntry _credit(String role, [String? character]) =>
    FilmographyEntry.fromJson(<String, dynamic>{
      'title_id': 'm1',
      'kind': 'movie',
      'display_title': 'Alpha',
      'role': role,
      'character': character,
    });

void main() {
  test('one credit reads as its role and character', () {
    expect(
      filmographyCaption(<FilmographyEntry>[_credit('actor', 'Hero')]),
      '${Strings.roleActor} · Hero',
    );
    expect(
      filmographyCaption(<FilmographyEntry>[_credit('director')]),
      Strings.roleDirector,
    );
  });

  test('every role on a title is listed once, with all its characters', () {
    expect(
      filmographyCaption(<FilmographyEntry>[
        _credit('actor', 'Hero'),
        _credit('director'),
        _credit('actor', 'Villain'),
        _credit('writer', ''),
        _credit('actor', 'Hero'),
      ]),
      '${Strings.roleActor} · Hero / Villain, '
      '${Strings.roleDirector}, ${Strings.roleWriter}',
    );
  });
}
