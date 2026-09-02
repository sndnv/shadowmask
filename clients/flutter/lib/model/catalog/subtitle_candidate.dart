import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/catalog/version_detail.dart';

part 'subtitle_candidate.freezed.dart';
part 'subtitle_candidate.g.dart';

@freezed
abstract class SubtitleCandidate with _$SubtitleCandidate {
  const factory SubtitleCandidate({
    required String fileId,
    String? language,
    String? releaseName,
    required SubtitleFormat format,
    int? downloadCount,
    double? rating,
  }) = _SubtitleCandidate;

  factory SubtitleCandidate.fromJson(Map<String, dynamic> json) =>
      _$SubtitleCandidateFromJson(json);
}
