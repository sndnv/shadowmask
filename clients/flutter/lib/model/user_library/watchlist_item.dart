import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/title_ref.dart';

part 'watchlist_item.freezed.dart';
part 'watchlist_item.g.dart';

@freezed
abstract class WatchlistItem with _$WatchlistItem {
  const factory WatchlistItem({
    required String userId,
    required TitleRef title,
    required String addedAt,
  }) = _WatchlistItem;

  factory WatchlistItem.fromJson(Map<String, dynamic> json) =>
      _$WatchlistItemFromJson(json);
}
