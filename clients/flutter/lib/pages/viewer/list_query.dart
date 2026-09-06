import 'package:shadowmask/nav/route_args.dart';
import 'package:shadowmask/pages/viewer/list_prefs_store.dart';

int? pageSizeArg(Map<String, String> args) {
  final int? limit = int.tryParse(args['limit'] ?? '');
  return limit != null && limit > 0 ? limit : null;
}

class ListQuery {
  const ListQuery({
    required this.offset,
    required this.sort,
    required this.order,
    required this.genres,
    this.limit,
    this.library,
    this.sortFromUrl = false,
    this.orderFromUrl = false,
  });

  final int offset;
  final String sort;
  final String order;
  final List<String> genres;
  final int? limit;
  final String? library;
  final bool sortFromUrl;
  final bool orderFromUrl;

  ListQuery withStored(ListOrdering? stored) => stored == null
      ? this
      : ListQuery(
          offset: offset,
          sort: sortFromUrl ? sort : stored.sort,
          order: orderFromUrl ? order : stored.order,
          genres: genres,
          limit: limit,
          library: library,
          sortFromUrl: sortFromUrl,
          orderFromUrl: orderFromUrl,
        );

  factory ListQuery.fromArgs(Map<String, String> q) {
    final String? library = q['library'];
    return ListQuery(
      offset: offsetArg(q),
      limit: pageSizeArg(q),
      sort: q['sort'] ?? 'added_at',
      order: q['order'] ?? 'asc',
      sortFromUrl: q['sort'] != null,
      orderFromUrl: q['order'] != null,
      genres: (q['genres'] ?? '')
          .split(',')
          .map((String s) => s.trim())
          .where((String s) => s.isNotEmpty)
          .toList(),
      library: (library != null && library.isNotEmpty) ? library : null,
    );
  }
}
