import 'package:shadowmask/model/catalog/series.dart';
import 'package:shadowmask/view/catalog_card.dart';

class Hub {
  const Hub({required this.id, required this.title, required this.items});

  final String id;
  final String title;
  final List<CatalogCard> items;

  factory Hub.fromJson(Map<String, dynamic> json) => Hub(
    id: json['id'] as String? ?? '',
    title: json['title'] as String? ?? '',
    items: ((json['items'] as List<dynamic>?) ?? <dynamic>[])
        .map((dynamic e) => _card(e as Map<String, dynamic>))
        .toList(growable: false),
  );

  static CatalogCard _card(Map<String, dynamic> json) {
    final Object? count = json['episode_count'];
    if (json['type'] == 'series' && count is num) {
      return CatalogCard.fromHubSeries(Series.fromJson(json), count.toInt());
    }
    return CatalogCard.fromJson(json, asSeriesPoster: true);
  }
}
