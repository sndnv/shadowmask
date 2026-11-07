import 'package:shadowmask/view/catalog_card.dart';

class QueueEntry {
  const QueueEntry({required this.id, required this.card});

  final String id;
  final CatalogCard card;

  factory QueueEntry.fromJson(Map<String, dynamic> json) {
    final String id = json['id'] as String;
    final CatalogCard card = CatalogCard.fromJson(
      json['card'] as Map<String, dynamic>,
      asSeriesPoster: true,
    );
    return QueueEntry(id: id, card: card.queued(id));
  }
}
