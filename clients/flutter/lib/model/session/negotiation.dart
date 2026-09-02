import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';

part 'negotiation.freezed.dart';
part 'negotiation.g.dart';

@freezed
abstract class Negotiation with _$Negotiation {
  const factory Negotiation({
    @Default('') String manifestUrl,
    @Default(PlaybackMode.direct) PlaybackMode mode,
    SelectedTracks? selected,
  }) = _Negotiation;

  factory Negotiation.fromJson(Map<String, dynamic> json) =>
      _$NegotiationFromJson(json);
}
