import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/title_ref.dart';

part 'resolve_candidate.freezed.dart';
part 'resolve_candidate.g.dart';

@freezed
abstract class ResolveTarget with _$ResolveTarget {
  const factory ResolveTarget({
    required String kind,
    TitleRef? title,
    String? source,
    String? value,
  }) = _ResolveTarget;

  factory ResolveTarget.fromJson(Map<String, dynamic> json) =>
      _$ResolveTargetFromJson(json);
}

@freezed
abstract class ResolveCandidate with _$ResolveCandidate {
  const factory ResolveCandidate({
    required String source,
    required ResolveTarget target,
    required String title,
    int? year,
    required String kind,
  }) = _ResolveCandidate;

  factory ResolveCandidate.fromJson(Map<String, dynamic> json) =>
      _$ResolveCandidateFromJson(json);
}
