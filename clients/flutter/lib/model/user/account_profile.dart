import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/content_rating.dart';
import 'package:shadowmask/model/user/self_user.dart';

part 'account_profile.freezed.dart';
part 'account_profile.g.dart';

@freezed
abstract class AccountProfile with _$AccountProfile {
  const factory AccountProfile({
    required String id,
    required String username,
    required UserRole role,
    ContentRating? maxContentRating,
    @Default(<String>[]) List<String> preferredAudio,
    @Default(<String>[]) List<String> preferredSubtitle,
    int? concurrentStreamLimit,
    int? bitrateCap,
    @Default(true) bool active,
    required String createdAt,
    required String updatedAt,
  }) = _AccountProfile;

  factory AccountProfile.fromJson(Map<String, dynamic> json) =>
      _$AccountProfileFromJson(json);
}
