import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/title_ref.dart';

part 'duplicate_candidate.freezed.dart';
part 'duplicate_candidate.g.dart';

enum DuplicateKind {
  @JsonValue('duplicate')
  duplicate,
  @JsonValue('multi_part')
  multiPart,
}

@freezed
abstract class DuplicateCandidate with _$DuplicateCandidate {
  const factory DuplicateCandidate({
    required String id,
    required TitleRef title,
    @Default(DuplicateKind.duplicate) DuplicateKind kind,
    @Default(<String>[]) List<String> paths,
  }) = _DuplicateCandidate;

  factory DuplicateCandidate.fromJson(Map<String, dynamic> json) =>
      _$DuplicateCandidateFromJson(json);
}
