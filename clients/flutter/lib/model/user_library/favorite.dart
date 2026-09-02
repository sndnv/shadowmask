import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/title_ref.dart';

part 'favorite.freezed.dart';
part 'favorite.g.dart';

@freezed
abstract class Favorite with _$Favorite {
  const factory Favorite({
    required String userId,
    required TitleRef title,
    required String addedAt,
  }) = _Favorite;

  factory Favorite.fromJson(Map<String, dynamic> json) =>
      _$FavoriteFromJson(json);
}
