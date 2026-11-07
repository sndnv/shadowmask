import 'package:shadowmask/view/catalog_card.dart';

class PlaylistItem {
  const PlaylistItem({required this.position, required this.card});

  final int position;
  final CatalogCard card;

  factory PlaylistItem.fromJson(Map<String, dynamic> json) => PlaylistItem(
    position: json['position'] as int,
    card: CatalogCard.fromJson(
      json['card'] as Map<String, dynamic>,
      asSeriesPoster: true,
    ),
  );
}
