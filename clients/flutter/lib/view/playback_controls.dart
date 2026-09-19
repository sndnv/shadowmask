import 'package:shadowmask/model/session/selected_tracks.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';

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

class PlaybackControls {
  const PlaybackControls({
    this.height,
    this.burn = false,
    this.downmix = false,
    this.audioTrack,
    this.audioLanguage,
    this.subtitle,
    this.subtitleLanguage,
    this.subtitleOff = false,
    this.offsetMs = 0,
    this.delivery = DeliveryPreference.auto,
  });

  factory PlaybackControls.fromQuery(Map<String, String> q) {
    int? asInt(String? v) =>
        (v != null && v.isNotEmpty) ? int.tryParse(v) : null;
    String? asText(String? v) => (v != null && v.isNotEmpty) ? v : null;
    return PlaybackControls(
      height: asInt(q['height']),
      burn: q['burn'] == '1',
      downmix: q['downmix'] == '1',
      audioTrack: asInt(q['audio']),
      audioLanguage: asText(q['alang']),
      subtitle: SubtitleSelection.fromWire(q['sub']),
      subtitleLanguage: asText(q['slang']),
      subtitleOff: q['soff'] == '1',
      offsetMs: asInt(q['offset']) ?? 0,
      delivery: DeliveryPreference.fromWire(q['delivery']),
    );
  }

  final int? height;
  final bool burn;
  final bool downmix;
  final int? audioTrack;
  final String? audioLanguage;
  final SubtitleSelection? subtitle;
  final String? subtitleLanguage;
  final bool subtitleOff;
  final int offsetMs;
  final DeliveryPreference delivery;

  bool get isDefault =>
      height == null &&
      !burn &&
      !downmix &&
      offsetMs == 0 &&
      delivery == DeliveryPreference.auto &&
      !hasTrackRequest;

  bool get hasTrackRequest =>
      audioTrack != null ||
      audioLanguage != null ||
      subtitle != null ||
      subtitleLanguage != null ||
      subtitleOff;

  PlaybackControls withSelection(SelectedTracks? selected) => PlaybackControls(
    height: height,
    burn: burn,
    downmix: downmix,
    audioTrack: selected?.audioTrack ?? audioTrack,
    audioLanguage: audioLanguage,
    subtitle: selected?.subtitleTrack ?? subtitle,
    subtitleLanguage: subtitleLanguage,
    subtitleOff: subtitleOff,
    offsetMs: offsetMs,
    delivery: delivery,
  );

  Map<String, dynamic> toStartBody() => <String, dynamic>{
    if (height != null) 'target_height': height,
    if (burn) 'force_burn': true,
    if (downmix) 'downmix_stereo': true,
    if (audioTrack != null) 'audio_track': audioTrack,
    if (audioLanguage != null) 'audio_language': audioLanguage,
    if (subtitle != null)
      'subtitle': <String, dynamic>{
        'track': subtitle!.toJson(),
        if (offsetMs != 0) 'offset_ms': offsetMs,
      },
    if (subtitleLanguage != null) 'subtitle_language': subtitleLanguage,
    if (subtitleOff) 'subtitle_off': true,
    if (delivery != DeliveryPreference.auto) 'delivery': delivery.wire,
  };

  Map<String, dynamic> toUpdateBody() {
    final Map<String, dynamic> body = <String, dynamic>{
      'target_height': height,
      'force_burn': burn,
      'downmix_stereo': downmix,
      'delivery': delivery.wire,
    };
    if (audioTrack != null) {
      body['audio_track'] = audioTrack;
    }
    if (subtitle != null) {
      body['subtitle'] = <String, dynamic>{
        'action': 'set',
        'track': subtitle!.toJson(),
        if (offsetMs != 0) 'offset_ms': offsetMs,
      };
    } else {
      body['subtitle'] = <String, dynamic>{'action': 'disable'};
    }
    return body;
  }

  Map<String, String?> toQuery() => <String, String?>{
    if (height != null) 'height': height.toString(),
    if (burn) 'burn': '1',
    if (downmix) 'downmix': '1',
    if (audioTrack != null) 'audio': audioTrack.toString(),
    if (audioLanguage != null) 'alang': audioLanguage,
    if (subtitle != null) 'sub': subtitle!.toWire(),
    if (subtitleLanguage != null) 'slang': subtitleLanguage,
    if (subtitleOff) 'soff': '1',
    if (offsetMs != 0) 'offset': offsetMs.toString(),
    if (delivery != DeliveryPreference.auto) 'delivery': delivery.wire,
  };

  @override
  bool operator ==(Object other) =>
      other is PlaybackControls &&
      other.height == height &&
      other.burn == burn &&
      other.downmix == downmix &&
      other.audioTrack == audioTrack &&
      other.audioLanguage == audioLanguage &&
      other.subtitle == subtitle &&
      other.subtitleLanguage == subtitleLanguage &&
      other.subtitleOff == subtitleOff &&
      other.offsetMs == offsetMs &&
      other.delivery == delivery;

  @override
  int get hashCode => Object.hash(
    height,
    burn,
    downmix,
    audioTrack,
    audioLanguage,
    subtitle,
    subtitleLanguage,
    subtitleOff,
    offsetMs,
    delivery,
  );
}
