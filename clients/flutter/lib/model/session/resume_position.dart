import 'package:freezed_annotation/freezed_annotation.dart';

part 'resume_position.freezed.dart';
part 'resume_position.g.dart';

@freezed
abstract class ResumePosition with _$ResumePosition {
  const factory ResumePosition({@Default(0) int positionMs}) = _ResumePosition;

  factory ResumePosition.fromJson(Map<String, dynamic> json) =>
      _$ResumePositionFromJson(json);
}
