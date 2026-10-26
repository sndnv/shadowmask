import 'package:freezed_annotation/freezed_annotation.dart';

part 'shared_with.freezed.dart';
part 'shared_with.g.dart';

@freezed
abstract class SharedWith with _$SharedWith {
  const factory SharedWith({required String userId, required String username}) =
      _SharedWith;

  factory SharedWith.fromJson(Map<String, dynamic> json) =>
      _$SharedWithFromJson(json);
}
