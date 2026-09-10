import 'package:web/web.dart' as web;

Future<bool> startDownload(String url, String filename) async {
  final web.HTMLAnchorElement anchor =
      web.document.createElement('a') as web.HTMLAnchorElement;
  anchor.href = url;
  anchor.download = filename;
  anchor.style.display = 'none';
  web.document.body?.appendChild(anchor);
  anchor.click();
  anchor.remove();
  return true;
}
