import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/util/external_id.dart';

void main() {
  test('blank input means no forced match', () {
    expect(externalIdFor('', TitleKind.movie), isNull);
    expect(externalIdFor('   ', TitleKind.series), isNull);
  });

  test('a tt prefix is an IMDb id, whatever the title kind', () {
    expect(externalIdFor('tt0133093', TitleKind.movie), <String, String>{
      'source': 'imdb',
      'value': 'tt0133093',
    });
    expect(externalIdFor('TT0903747', TitleKind.series), <String, String>{
      'source': 'imdb',
      'value': 'TT0903747',
    });
  });

  test('a bare number takes the TMDB prefix of its kind', () {
    expect(externalIdFor(' 603 ', TitleKind.movie), <String, String>{
      'source': 'tmdb',
      'value': 'movie/603',
    });
    expect(externalIdFor('1396', TitleKind.series), <String, String>{
      'source': 'tmdb',
      'value': 'tv/1396',
    });
  });

  test('a full TMDB path is passed through untouched', () {
    expect(externalIdFor('tv/1396', TitleKind.movie), <String, String>{
      'source': 'tmdb',
      'value': 'tv/1396',
    });
  });
}
