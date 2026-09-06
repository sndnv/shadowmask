String? normalizeServerAddress(String input) {
  final String trimmed = input.trim();
  if (trimmed.isEmpty) {
    return null;
  }
  final Uri? uri = Uri.tryParse(
    trimmed.contains('://') ? trimmed : 'http://$trimmed',
  );
  if (uri == null || uri.host.isEmpty) {
    return null;
  }
  if (!uri.isScheme('http') && !uri.isScheme('https')) {
    return null;
  }
  String path = uri.path;
  while (path.endsWith('/')) {
    path = path.substring(0, path.length - 1);
  }
  return Uri(
    scheme: uri.scheme,
    host: uri.host,
    port: uri.hasPort ? uri.port : null,
    path: path,
  ).toString();
}
