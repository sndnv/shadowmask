Map<String, String> routeArgs(Map<String, List<String>> params) =>
    <String, String>{
      for (final MapEntry<String, List<String>> entry in params.entries)
        if (entry.value.isNotEmpty) entry.key: entry.value.first,
    };

int offsetArg(Map<String, String> args) =>
    int.tryParse(args['offset'] ?? '') ?? 0;

int? startAtArg(Map<String, String> args) {
  final int? at = int.tryParse(args['at'] ?? '');
  return at != null && at >= 0 ? at : null;
}
