import 'package:flutter/widgets.dart';
import 'package:intl/intl.dart';

const int kPlaybackLogLimit = 500;
const int kOverlayLogLines = 8;
const int kOverlayLineLength = 100;

final DateFormat _clock = DateFormat('HH:mm:ss');

class PlaybackLog {
  PlaybackLog({DateTime Function()? now}) : _now = now ?? DateTime.now;

  final DateTime Function() _now;
  final List<String> _lines = <String>[];

  void add(String message) {
    _lines.add('${_clock.format(_now())} $message');
    if (_lines.length > kPlaybackLogLimit) {
      _lines.removeRange(0, _lines.length - kPlaybackLogLimit);
    }
  }

  List<String> get lines => List<String>.unmodifiable(_lines);

  List<String> recent([int count = kOverlayLogLines]) =>
      List<String>.unmodifiable(
        _lines.length <= count ? _lines : _lines.sublist(_lines.length - count),
      );
}

String playbackEventLine(
  String event, {
  required double positionS,
  required double durationS,
  required List<(double, double)> buffered,
  required bool hidden,
}) {
  final String duration = durationS.isFinite
      ? durationS.toStringAsFixed(1)
      : '?';
  final String ranges = buffered.isEmpty
      ? 'none'
      : buffered
            .map(
              ((double, double) r) =>
                  '${r.$1.toStringAsFixed(1)}-${r.$2.toStringAsFixed(1)}',
            )
            .join(' ');
  return '$event at ${positionS.toStringAsFixed(1)}/$duration s, '
      'buffered $ranges${hidden ? ', hidden' : ''}';
}

String clipLine(String line, [int length = kOverlayLineLength]) {
  final Characters chars = line.characters;
  return chars.length <= length ? line : '${chars.take(length)}…';
}
