import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/session/selected_tracks.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_choice.dart';

part 'playback_controls.freezed.dart';

enum DeliveryPreference {
  auto('auto'),
  never('never'),
  always('always');

  const DeliveryPreference(this.wire);

  final String wire;

  static DeliveryPreference fromWire(String? value) => values.firstWhere(
    (DeliveryPreference p) => p.wire == value,
    orElse: () => DeliveryPreference.auto,
  );
}

@freezed
abstract class PlaybackControls with _$PlaybackControls {
  const PlaybackControls._();

  const factory PlaybackControls({
    int? height,
    @Default(false) bool burn,
    bool? downmix,
    bool? normalize,
    DialogueLift? dialogue,
    NightMode? night,
    int? audioTrack,
    String? audioLanguage,
    @Default(SubtitleChoice.auto()) SubtitleChoice subtitle,
    String? subtitleLanguage,
    @Default(0) int offsetMs,
    DeliveryPreference? delivery,
  }) = _PlaybackControls;

  factory PlaybackControls.fromQuery(Map<String, String> q) {
    int? asInt(String? v) =>
        (v != null && v.isNotEmpty) ? int.tryParse(v) : null;
    String? asText(String? v) => (v != null && v.isNotEmpty) ? v : null;
    bool? asSwitch(String? v) => v == null ? null : v == '1';
    final SubtitleSelection? track = SubtitleSelection.fromWire(q['sub']);
    return PlaybackControls(
      height: asInt(q['height']),
      burn: q['burn'] == '1',
      downmix: asSwitch(q['downmix']),
      normalize: asSwitch(q['normalize']),
      dialogue: q.containsKey('dialogue')
          ? DialogueLift.fromDb(asInt(q['dialogue']))
          : null,
      night: q.containsKey('night') ? NightMode.fromWire(q['night']) : null,
      audioTrack: asInt(q['audio']),
      audioLanguage: asText(q['alang']),
      subtitle: track != null
          ? SubtitleChoice.on(track)
          : q['soff'] == '1'
          ? const SubtitleChoice.off()
          : const SubtitleChoice.auto(),
      subtitleLanguage: asText(q['slang']),
      offsetMs: asInt(q['offset']) ?? 0,
      delivery: q.containsKey('delivery')
          ? DeliveryPreference.fromWire(q['delivery'])
          : null,
    );
  }

  bool get hasTrackRequest =>
      audioTrack != null ||
      audioLanguage != null ||
      subtitle is! SubtitleAuto ||
      subtitleLanguage != null;

  PlaybackControls withSelection(SelectedTracks? selected) {
    final SubtitleSelection? track = selected?.subtitleTrack;
    return copyWith(
      audioTrack: selected?.audioTrack ?? audioTrack,
      subtitle: track != null ? SubtitleChoice.on(track) : subtitle,
    );
  }

  PlaybackControls withDefaultDelivery(DeliveryPreference fallback) =>
      delivery != null ? this : copyWith(delivery: fallback);

  PlaybackControls withDefaultSound({
    required bool downmix,
    required bool normalize,
    required DialogueLift dialogue,
    required NightMode night,
  }) => copyWith(
    downmix: this.downmix ?? downmix,
    normalize: this.normalize ?? normalize,
    dialogue: this.dialogue ?? dialogue,
    night: this.night ?? night,
  );

  bool sameTuningAs(PlaybackControls other) =>
      (normalize ?? true) == (other.normalize ?? true) &&
      (dialogue ?? DialogueLift.none) == (other.dialogue ?? DialogueLift.none);

  PlaybackControls get withoutTuning =>
      copyWith(normalize: null, dialogue: null);

  Map<String, dynamic> toStartBody() => <String, dynamic>{
    if (height != null) 'target_height': height,
    if (burn) 'force_burn': true,
    if (downmix ?? false) 'downmix_stereo': true,
    if (normalize == false) 'normalize_downmix': false,
    if (dialogue != null && dialogue != DialogueLift.none)
      'dialogue_db': dialogue!.db,
    if (night != null && night != NightMode.off) 'night': night!.wire,
    if (audioTrack != null) 'audio_track': audioTrack,
    if (audioLanguage != null) 'audio_language': audioLanguage,
    if (subtitle.selection case final SubtitleSelection track)
      'subtitle': <String, dynamic>{
        'track': track.toJson(),
        if (offsetMs != 0) 'offset_ms': offsetMs,
      },
    if (subtitleLanguage != null) 'subtitle_language': subtitleLanguage,
    if (subtitle is SubtitleOff) 'subtitle_off': true,
    if (delivery != null && delivery != DeliveryPreference.auto)
      'delivery': delivery!.wire,
  };

  Map<String, dynamic> toUpdateBody() {
    final Map<String, dynamic> body = <String, dynamic>{
      'target_height': height,
      'force_burn': burn,
      'downmix_stereo': downmix ?? false,
      'normalize_downmix': normalize ?? true,
      'dialogue_db': (dialogue ?? DialogueLift.none).db,
      'night': (night ?? NightMode.off).wire,
      'delivery': ?delivery?.wire,
    };
    if (audioTrack != null) {
      body['audio_track'] = audioTrack;
    }
    final SubtitleSelection? track = subtitle.selection;
    if (track != null) {
      body['subtitle'] = <String, dynamic>{
        'action': 'set',
        'track': track.toJson(),
        if (offsetMs != 0) 'offset_ms': offsetMs,
      };
    } else {
      body['subtitle'] = <String, dynamic>{'action': 'disable'};
    }
    return body;
  }

  bool sameUpdateAs(PlaybackControls other) => const DeepCollectionEquality()
      .equals(toUpdateBody(), other.toUpdateBody());

  Map<String, String?> toQuery() => <String, String?>{
    if (height != null) 'height': height.toString(),
    if (burn) 'burn': '1',
    if (downmix != null) 'downmix': downmix! ? '1' : '0',
    if (normalize != null) 'normalize': normalize! ? '1' : '0',
    if (dialogue != null) 'dialogue': '${dialogue!.db}',
    if (night != null) 'night': night!.wire,
    if (audioTrack != null) 'audio': audioTrack.toString(),
    if (audioLanguage != null) 'alang': audioLanguage,
    if (subtitle.selection case final SubtitleSelection track)
      'sub': track.toWire(),
    if (subtitleLanguage != null) 'slang': subtitleLanguage,
    if (subtitle is SubtitleOff) 'soff': '1',
    if (offsetMs != 0) 'offset': offsetMs.toString(),
    if (delivery != null) 'delivery': delivery!.wire,
  };
}
