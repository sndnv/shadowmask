import 'package:freezed_annotation/freezed_annotation.dart';

part 'content_rating.freezed.dart';
part 'content_rating.g.dart';

@freezed
abstract class ContentRating with _$ContentRating {
  const ContentRating._();

  const factory ContentRating({required String system, required String code}) =
      _ContentRating;

  factory ContentRating.fromJson(Map<String, dynamic> json) =>
      _$ContentRatingFromJson(json);

  String get label => '${system.toUpperCase()} ${code.toUpperCase()}';
}
