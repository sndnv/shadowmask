String absoluteUrl(String baseUrl, String path) {
  final String base = baseUrl.endsWith('/')
      ? baseUrl.substring(0, baseUrl.length - 1)
      : baseUrl;
  return path.startsWith('/') ? '$base$path' : '$base/$path';
}
