import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/title_ref.dart';

part 'title_row.freezed.dart';
part 'title_row.g.dart';

enum TitleAvailability {
  available('available'),
  unavailable('unavailable'),
  noVersion('no_version');

  const TitleAvailability(this.wire);

  final String wire;
}

enum TitleSort {
  title('title'),
  kind('kind'),
  year('year'),
  versions('versions'),
  added('added');

  const TitleSort(this.wire);

  final String wire;
}

@freezed
abstract class TitleRow with _$TitleRow {
  const factory TitleRow({
    required TitleKind kind,
    required String id,
    String? seriesId,
    String? seasonId,
    String? title,
    String? seriesTitle,
    int? seasonNumber,
    int? episodeNumber,
    int? year,
    @Default(0) int files,
    int? episodes,
    int? seasons,
    @Default(false) bool available,
    @Default(false) bool readable,
    String? addedAt,
  }) = _TitleRow;

  factory TitleRow.fromJson(Map<String, dynamic> json) =>
      _$TitleRowFromJson(json);
}
