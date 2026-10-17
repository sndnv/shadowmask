import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/session/subtitle_selection.dart';

part 'subtitle_choice.freezed.dart';

@freezed
sealed class SubtitleChoice with _$SubtitleChoice {
  const SubtitleChoice._();

  const factory SubtitleChoice.auto() = SubtitleAuto;

  const factory SubtitleChoice.off() = SubtitleOff;

  const factory SubtitleChoice.on(SubtitleSelection track) = SubtitleOn;

  SubtitleSelection? get selection => switch (this) {
    SubtitleOn(:final SubtitleSelection track) => track,
    SubtitleAuto() || SubtitleOff() => null,
  };
}
