import 'package:package_info_plus/package_info_plus.dart';

typedef VersionLoader = Future<String> Function();

Future<String> defaultVersion() async =>
    (await PackageInfo.fromPlatform()).version;
