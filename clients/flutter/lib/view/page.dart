class Paged<T> {
  const Paged({
    required this.items,
    required this.total,
    required this.offset,
    required this.limit,
  });

  final List<T> items;
  final int total;
  final int offset;
  final int limit;

  factory Paged.fromJson(
    Map<String, dynamic> json,
    T Function(Map<String, dynamic>) item,
  ) {
    final List<dynamic> raw = (json['items'] as List<dynamic>?) ?? <dynamic>[];
    return Paged<T>(
      items: raw
          .map((dynamic e) => item(e as Map<String, dynamic>))
          .toList(growable: false),
      total: (json['total'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 0,
    );
  }

  bool get isEmpty => items.isEmpty;
  int get end => offset + items.length;
  bool get hasPrev => offset > 0;
  bool get hasNext => end < total;
}
