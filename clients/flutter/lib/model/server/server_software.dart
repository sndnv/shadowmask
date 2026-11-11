import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/server/server_feature.dart';

part 'server_software.freezed.dart';
part 'server_software.g.dart';

@freezed
abstract class ServerSoftware with _$ServerSoftware {
  const factory ServerSoftware({
    @Default('') String version,
    String? ytDlp,
    @Default(<ServerFeature>[]) List<ServerFeature> features,
  }) = _ServerSoftware;

  factory ServerSoftware.fromJson(Map<String, dynamic> json) =>
      _$ServerSoftwareFromJson(json);
}
