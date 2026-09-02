import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/title_ref.dart';

part 'watched_rollup.freezed.dart';
part 'watched_rollup.g.dart';

@freezed
abstract class WatchedRollup with _$WatchedRollup {
  const factory WatchedRollup({
    required TitleRef target,
    @Default(false) bool watched,
    @Default(false) bool completed,
    @Default(0) int watchedEpisodes,
    @Default(0) int totalEpisodes,
  }) = _WatchedRollup;

  factory WatchedRollup.fromJson(Map<String, dynamic> json) =>
      _$WatchedRollupFromJson(json);
}
