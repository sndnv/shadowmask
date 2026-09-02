String pad2(int n) => n.toString().padLeft(2, '0');

String episodeCode(int? season, int number) =>
    season != null ? 'S${pad2(season)}E${pad2(number)}' : 'E${pad2(number)}';

String megabytes(int bytes) => '${(bytes / 1048576).round()} MB';

String runtime(int minutes) => '$minutes min';

String scoreSource(String source) => switch (source.toLowerCase()) {
  'internet movie database' => 'IMDb',
  _ => source,
};

String scoreText(String source, double value) {
  final String number = value == value.roundToDouble()
      ? value.toStringAsFixed(0)
      : value.toStringAsFixed(1);
  return switch (source.toLowerCase()) {
    'internet movie database' => '$number/10',
    'rotten tomatoes' => '$number%',
    'metacritic' => '$number/100',
    _ => number,
  };
}

String durationText(int ms) {
  final int totalMinutes = ms ~/ 60000;
  final int hours = totalMinutes ~/ 60;
  final int minutes = totalMinutes % 60;
  return hours > 0 ? '${hours}h ${pad2(minutes)}m' : '${minutes}m';
}

String clock(int ms) {
  final int total = (ms < 0 ? 0 : ms) ~/ 1000;
  final int hours = total ~/ 3600;
  final int minutes = (total % 3600) ~/ 60;
  final int seconds = total % 60;
  return hours > 0
      ? '$hours:${pad2(minutes)}:${pad2(seconds)}'
      : '$minutes:${pad2(seconds)}';
}

String _ymd(DateTime d) => '${d.year}-${pad2(d.month)}-${pad2(d.day)}';

String? dateText(String? iso) {
  if (iso == null || iso.isEmpty) {
    return null;
  }
  final DateTime? parsed = DateTime.tryParse(iso);
  if (parsed == null) {
    return iso;
  }
  return _ymd(parsed.toLocal());
}

String? dateTimeText(String? iso) {
  if (iso == null || iso.isEmpty) {
    return null;
  }
  final DateTime? parsed = DateTime.tryParse(iso);
  if (parsed == null) {
    return iso;
  }
  final DateTime d = parsed.toLocal();
  return '${_ymd(d)} ${pad2(d.hour)}:${pad2(d.minute)}';
}

String? relativeText(String? iso, {DateTime? now}) {
  if (iso == null || iso.isEmpty) {
    return null;
  }
  final DateTime? parsed = DateTime.tryParse(iso);
  if (parsed == null) {
    return null;
  }
  final DateTime at = parsed.toLocal();
  final DateTime reference = now ?? DateTime.now();
  final Duration since = reference.difference(at);
  if (since.isNegative) {
    return 'in the future';
  }
  if (since.inSeconds < 60) {
    return 'just now';
  }
  if (since.inMinutes < 60) {
    return _ago(since.inMinutes, 'minute');
  }
  if (since.inHours < 24) {
    return _ago(since.inHours, 'hour');
  }
  if (since.inDays < 31) {
    return _ago(since.inDays, 'day');
  }
  if (since.inDays < 365) {
    return _ago(since.inDays ~/ 30, 'month');
  }
  return _ago(since.inDays ~/ 365, 'year');
}

String _ago(int count, String unit) =>
    count == 1 ? '1 $unit ago' : '$count ${unit}s ago';
