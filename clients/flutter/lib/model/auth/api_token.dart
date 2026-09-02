import 'package:freezed_annotation/freezed_annotation.dart';

part 'api_token.freezed.dart';
part 'api_token.g.dart';

@freezed
abstract class ApiToken with _$ApiToken {
  const factory ApiToken({
    required String id,
    required String userId,
    required String deviceId,
    required String createdAt,
    String? lastUsedAt,
  }) = _ApiToken;

  factory ApiToken.fromJson(Map<String, dynamic> json) =>
      _$ApiTokenFromJson(json);
}
