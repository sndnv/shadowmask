import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/title_ref.dart';

part 'watch_history.freezed.dart';
part 'watch_history.g.dart';

@freezed
abstract class WatchHistory with _$WatchHistory {
  const factory WatchHistory({
    required String userId,
    required TitleRef title,
    @Default(false) bool watched,
    @Default(0) int playCount,
    String? lastWatchedAt,
    @Default(false) bool completed,
  }) = _WatchHistory;

  factory WatchHistory.fromJson(Map<String, dynamic> json) =>
      _$WatchHistoryFromJson(json);
}
