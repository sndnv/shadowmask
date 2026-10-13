import 'package:shadowmask/model/catalog/person_profile.dart';
import 'package:shadowmask/model/common/title_ref.dart';

List<List<FilmographyEntry>> filmographyByTitle(
  List<FilmographyEntry> filmography,
) {
  final Map<(TitleKind, String), List<FilmographyEntry>> titles =
      <(TitleKind, String), List<FilmographyEntry>>{};
  for (final FilmographyEntry entry in filmography) {
    titles
        .putIfAbsent((entry.kind, entry.titleId), () => <FilmographyEntry>[])
        .add(entry);
  }
  return titles.values.toList();
}
