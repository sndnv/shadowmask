import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/server/library_prep.dart';

part 'playback_prep_report.freezed.dart';
part 'playback_prep_report.g.dart';

@freezed
abstract class PlaybackPrepReport with _$PlaybackPrepReport {
  const factory PlaybackPrepReport({
    String? ffmpeg,
    @Default(<LibraryPrep>[]) List<LibraryPrep> libraries,
  }) = _PlaybackPrepReport;

  factory PlaybackPrepReport.fromJson(Map<String, dynamic> json) =>
      _$PlaybackPrepReportFromJson(json);
}
