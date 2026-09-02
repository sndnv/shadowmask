import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/common/title_ref.dart';

part 'resume_card.freezed.dart';
part 'resume_card.g.dart';

@freezed
abstract class ResumeCard with _$ResumeCard {
  const factory ResumeCard({
    required TitleRef title,
    required String displayTitle,
    Artwork? artwork,
    @Default(0) int durationMs,
    @Default(0) int progressPercent,
    int? year,
    String? seriesTitle,
    Artwork? seriesArtwork,
    int? seasonNumber,
    int? episodeNumber,
  }) = _ResumeCard;

  factory ResumeCard.fromJson(Map<String, dynamic> json) =>
      _$ResumeCardFromJson(json);
}
