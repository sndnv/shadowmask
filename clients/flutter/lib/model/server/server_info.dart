import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/server/rating_system.dart';

part 'server_info.freezed.dart';
part 'server_info.g.dart';

@freezed
abstract class ServerInfo with _$ServerInfo {
  const factory ServerInfo({
    @Default('') String version,
    @Default(<String>[]) List<String> features,
    @Default(1) int profileVersion,
    @Default(<RatingSystem>[]) List<RatingSystem> ratingSystems,
  }) = _ServerInfo;

  factory ServerInfo.fromJson(Map<String, dynamic> json) =>
      _$ServerInfoFromJson(json);
}
