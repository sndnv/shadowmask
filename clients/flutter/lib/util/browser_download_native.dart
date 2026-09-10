import 'package:url_launcher/url_launcher.dart';

Future<bool> startDownload(String url, String filename) async {
  final Uri? target = Uri.tryParse(url);
  if (target == null) {
    return false;
  }
  try {
    return await launchUrl(target, mode: LaunchMode.externalApplication);
  } catch (_) {
    return false;
  }
}
