import 'package:freezed_annotation/freezed_annotation.dart';

part 'random_pick.freezed.dart';
part 'random_pick.g.dart';

@freezed
abstract class RandomPick with _$RandomPick {
  const factory RandomPick({required String versionId}) = _RandomPick;

  factory RandomPick.fromJson(Map<String, dynamic> json) =>
      _$RandomPickFromJson(json);
}
