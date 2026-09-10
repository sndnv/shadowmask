import 'package:shadowmask/util/browser_download.dart' as platform;

typedef DownloadStarter = Future<bool> Function(String url, String filename);

DownloadStarter startDownload = platform.startDownload;
