import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/title_ref.dart';

part 'item_state.freezed.dart';
part 'item_state.g.dart';

@freezed
abstract class ItemState with _$ItemState {
  const factory ItemState({
    required TitleRef title,
    @Default(false) bool favorite,
    @Default(false) bool watchlisted,
    @Default(false) bool watched,
    @Default(false) bool completed,
    @Default(0) int progressPercent,
  }) = _ItemState;

  factory ItemState.fromJson(Map<String, dynamic> json) =>
      _$ItemStateFromJson(json);
}
