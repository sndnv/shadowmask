import 'package:shadowmask/model/common/title_ref.dart';

final RegExp _imdbId = RegExp(r'^tt\d+$', caseSensitive: false);

Map<String, String>? externalIdFor(String raw, TitleKind kind) {
  final String value = raw.trim();
  if (value.isEmpty) {
    return null;
  }
  if (_imdbId.hasMatch(value)) {
    return <String, String>{'source': 'imdb', 'value': value};
  }
  final String prefix = kind == TitleKind.series ? 'tv' : 'movie';
  return <String, String>{
    'source': 'tmdb',
    'value': value.contains('/') ? value : '$prefix/$value',
  };
}
