import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/title_ref.dart';

part 'unmatched_file.freezed.dart';
part 'unmatched_file.g.dart';

@freezed
abstract class MatchCandidate with _$MatchCandidate {
  const factory MatchCandidate({
    required TitleRef title,
    @Default(0) double confidence,
    @Default('') String label,
  }) = _MatchCandidate;

  factory MatchCandidate.fromJson(Map<String, dynamic> json) =>
      _$MatchCandidateFromJson(json);
}

@freezed
abstract class UnmatchedFile with _$UnmatchedFile {
  const factory UnmatchedFile({
    required String id,
    required String libraryId,
    required String path,
    @Default(<MatchCandidate>[]) List<MatchCandidate> candidates,
    required String createdAt,
    required String updatedAt,
  }) = _UnmatchedFile;

  factory UnmatchedFile.fromJson(Map<String, dynamic> json) =>
      _$UnmatchedFileFromJson(json);
}
