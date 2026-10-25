import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/model/session/negotiation.dart';
import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/model/session/playback_session.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';

const int kPlaybackHistoryLimit = 500;

class PlaybackHistory {
  PlaybackHistory({DateTime Function()? now}) : _now = now ?? DateTime.now;

  final DateTime Function() _now;
  final List<Map<String, Object?>> _entries = <Map<String, Object?>>[];

  void add(String event, [Map<String, Object?> facts = const {}]) {
    _entries.add(<String, Object?>{
      'at': _now().toUtc().toIso8601String(),
      'event': event,
      ...facts,
    });
    if (_entries.length > kPlaybackHistoryLimit) {
      _entries.removeRange(0, _entries.length - kPlaybackHistoryLimit);
    }
  }

  List<Map<String, Object?>> get entries =>
      List<Map<String, Object?>>.unmodifiable(_entries);
}

String _mode(PlaybackMode mode) => switch (mode) {
  PlaybackMode.direct => 'direct',
  PlaybackMode.remux => 'remux',
  PlaybackMode.transcode => 'transcode',
};

Map<String, Object?>? _tracks(SelectedTracks? selected) {
  if (selected == null) {
    return null;
  }
  return <String, Object?>{
    'audio_track': selected.audioTrack,
    'audio_delivery': selected.audioDelivery,
    'subtitle_track': selected.subtitleTrack?.toJson(),
    'subtitle_delivery': selected.subtitleDelivery,
  };
}

Map<String, Object?> startedFacts(PlaybackSession session) => <String, Object?>{
  'request': 'start',
  'session': session.sessionId,
  'mode': _mode(session.mode),
  'container': session.container,
  'origin_ms': session.originMs,
  'sequential': session.sequential,
  'selected': _tracks(session.selected),
};

Map<String, Object?> negotiatedFacts(
  String request,
  String sessionId,
  Negotiation negotiation,
) => <String, Object?>{
  'request': request,
  'session': sessionId,
  'mode': _mode(negotiation.mode),
  'container': negotiation.container,
  'origin_ms': negotiation.originMs,
  'sequential': negotiation.sequential,
  'selected': _tracks(negotiation.selected),
};

Map<String, Object?> failedFacts(String request, Object error) =>
    switch (error) {
      ApiException(:final int? status, :final String? code) =>
        <String, Object?>{
          'request': request,
          'status': status,
          'code': code,
          'message': error.message,
        },
      _ => <String, Object?>{'request': request, 'message': error.toString()},
    };
