import 'package:freezed_annotation/freezed_annotation.dart';

part 'self_user.freezed.dart';
part 'self_user.g.dart';

enum UserRole {
  @JsonValue('admin')
  admin,
  @JsonValue('user')
  user,
  @JsonValue('player')
  player,
  @JsonValue('automation')
  automation,
}

@freezed
abstract class SelfUser with _$SelfUser {
  const factory SelfUser({
    required String id,
    required String username,
    required UserRole role,
  }) = _SelfUser;

  factory SelfUser.fromJson(Map<String, dynamic> json) =>
      _$SelfUserFromJson(json);
}

extension SelfUserRoles on SelfUser {
  bool get isAdmin => role == UserRole.admin;
  bool get isPlayer => role == UserRole.player;
}
