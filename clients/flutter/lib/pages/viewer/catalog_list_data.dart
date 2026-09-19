import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/empty_state.dart';

class CatalogListData {
  const CatalogListData({
    required this.total,
    required this.offset,
    required this.genres,
    required this.cards,
    this.libraries = const <Library>[],
    this.empty,
  });

  final int total;
  final int offset;
  final List<Genre> genres;
  final List<CatalogCard> cards;
  final List<Library> libraries;
  final EmptyState? empty;

  CatalogListData withCards(List<CatalogCard> next) => CatalogListData(
    total: total,
    offset: offset,
    genres: genres,
    cards: next,
    libraries: libraries,
    empty: empty,
  );
}
