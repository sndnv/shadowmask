import 'package:freezed_annotation/freezed_annotation.dart';

part 'issued_token.freezed.dart';
part 'issued_token.g.dart';

@freezed
abstract class IssuedToken with _$IssuedToken {
  const factory IssuedToken({
    required String token,
    @JsonKey(name: 'expires_at') String? expiresAt,
    @JsonKey(name: 'device_id') @Default('') String deviceId,
  }) = _IssuedToken;

  factory IssuedToken.fromJson(Map<String, dynamic> json) =>
      _$IssuedTokenFromJson(json);
}
