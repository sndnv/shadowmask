import 'package:shadowmask/model/session/subtitle_selection.dart';

class PlaybackControls {
  const PlaybackControls({
    this.height,
    this.burn = false,
    this.downmix = false,
    this.audioTrack,
    this.subtitle,
    this.offsetMs = 0,
  });

  factory PlaybackControls.fromQuery(Map<String, String> q) {
    int? asInt(String? v) =>
        (v != null && v.isNotEmpty) ? int.tryParse(v) : null;
    return PlaybackControls(
      height: asInt(q['height']),
      burn: q['burn'] == '1',
      downmix: q['downmix'] == '1',
      audioTrack: asInt(q['audio']),
      subtitle: SubtitleSelection.fromWire(q['sub']),
      offsetMs: asInt(q['offset']) ?? 0,
    );
  }

  final int? height;
  final bool burn;
  final bool downmix;
  final int? audioTrack;
  final SubtitleSelection? subtitle;
  final int offsetMs;

  bool get isDefault =>
      height == null &&
      !burn &&
      !downmix &&
      audioTrack == null &&
      subtitle == null &&
      offsetMs == 0;

  Map<String, dynamic> toUpdateBody() {
    final Map<String, dynamic> body = <String, dynamic>{
      'target_height': height,
      'force_burn': burn,
      'downmix_stereo': downmix,
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
    if (subtitle != null) 'sub': subtitle!.toWire(),
    if (offsetMs != 0) 'offset': offsetMs.toString(),
  };

  @override
  bool operator ==(Object other) =>
      other is PlaybackControls &&
      other.height == height &&
      other.burn == burn &&
      other.downmix == downmix &&
      other.audioTrack == audioTrack &&
      other.subtitle == subtitle &&
      other.offsetMs == offsetMs;

  @override
  int get hashCode =>
      Object.hash(height, burn, downmix, audioTrack, subtitle, offsetMs);
}
