class PlayerDiagnostics {
  const PlayerDiagnostics(this.lines);

  final List<String> lines;
}

final RegExp _streamToken = RegExp(r'(/stream/)[^/\s]+(/)');

String redactStreamTokens(String text) => text.replaceAllMapped(
  _streamToken,
  (Match m) => '${m.group(1)}…${m.group(2)}',
);
