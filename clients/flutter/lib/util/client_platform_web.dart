import 'package:web/web.dart' as web;

String clientPlatform() {
  final String agent = web.window.navigator.userAgent;
  if (agent.contains('Chrome/') || agent.contains('Chromium/')) {
    return 'chrome';
  }
  if (agent.contains('Firefox/')) {
    return 'firefox';
  }
  if (agent.contains('Safari/')) {
    return 'safari';
  }
  return 'generic';
}
