import 'dart:io';

import 'package:path/path.dart' as p;
import 'package:xdg_directories/xdg_directories.dart' as xdg;

import 'package:shadowmask/l10n/strings.dart';

const String kDesktopEntryId = 'io.github.sndnv.shadowmask';
const String kBundledIconName = 'shadowmask';
const List<int> kDesktopIconSizes = <int>[64, 128, 256, 512];
const int kEntryIconSize = 512;
const String kInstallDirectory = 'shadowmask';
const String kInstalledAppImage = 'shadowmask.AppImage';

class DesktopEntry {
  const DesktopEntry({
    required this.appImage,
    required this.appDir,
    required this.dataHome,
  });

  final String appImage;
  final String appDir;
  final String dataHome;

  static DesktopEntry? fromEnvironment(
    Map<String, String> env, {
    required String Function() dataHome,
  }) {
    final String appImage = env['APPIMAGE'] ?? '';
    final String appDir = env['APPDIR'] ?? '';
    if (appImage.isEmpty || appDir.isEmpty) {
      return null;
    }
    final String home;
    try {
      home = dataHome();
    } catch (_) {
      return null;
    }
    return DesktopEntry(appImage: appImage, appDir: appDir, dataHome: home);
  }

  File get entryFile =>
      File(p.join(dataHome, 'applications', '$kDesktopEntryId.desktop'));

  File get installedAppImage =>
      File(p.join(dataHome, kInstallDirectory, kInstalledAppImage));

  File get _partialAppImage => File('${installedAppImage.path}.part');

  File iconFile(int size) => File(
    p.join(dataHome, 'icons', _hicolor(size), 'apps', '$kDesktopEntryId.png'),
  );

  File _bundledIcon(int size) => File(
    p.join(
      appDir,
      'usr',
      'share',
      'icons',
      _hicolor(size),
      'apps',
      '$kBundledIconName.png',
    ),
  );

  Future<bool> installed() => entryFile.exists();

  String get entryText =>
      desktopEntryText(installedAppImage.path, iconFile(kEntryIconSize).path);

  Future<bool> _runsFromCopy() async {
    final File copy = installedAppImage;
    if (!await copy.exists()) {
      return false;
    }
    return await File(appImage).resolveSymbolicLinks() ==
        await copy.resolveSymbolicLinks();
  }

  Future<void> add() async {
    if (!await _runsFromCopy()) {
      await installedAppImage.parent.create(recursive: true);
      await File(appImage).copy(_partialAppImage.path);
      await _partialAppImage.rename(installedAppImage.path);
    }
    for (final int size in kDesktopIconSizes) {
      final File icon = iconFile(size);
      await icon.parent.create(recursive: true);
      await _bundledIcon(size).copy(icon.path);
    }
    await entryFile.parent.create(recursive: true);
    await entryFile.writeAsString(entryText, flush: true);
  }

  Future<void> remove() async {
    for (final File file in <File>[
      entryFile,
      installedAppImage,
      _partialAppImage,
      for (final int size in kDesktopIconSizes) iconFile(size),
    ]) {
      if (await file.exists()) {
        await file.delete();
      }
    }
  }

  Future<void> refresh() async {
    if (!await installed()) {
      return;
    }
    if (!await _runsFromCopy() || await entryFile.readAsString() != entryText) {
      await add();
    }
  }
}

String _hicolor(int size) => p.join('hicolor', '${size}x$size');

String desktopEntryText(String appImage, String iconPath) => <String>[
  '[Desktop Entry]',
  'Type=Application',
  'Name=${Strings.appTitle}',
  'Exec=${_escapeValue(_quoteArgument(appImage))}',
  'TryExec=${_escapeValue(appImage)}',
  'Icon=${_escapeValue(iconPath)}',
  'Categories=AudioVideo;Video;',
  'StartupWMClass=$kDesktopEntryId',
  'Terminal=false',
  '',
].join('\n');

String _quoteArgument(String value) {
  final String escaped = value
      .replaceAllMapped(RegExp(r'["`$\\]'), (Match m) => '\\${m[0]}')
      .replaceAll('%', '%%');
  return '"$escaped"';
}

String _escapeValue(String value) => value
    .replaceAll(r'\', r'\\')
    .replaceAll('\n', r'\n')
    .replaceAll('\t', r'\t')
    .replaceAll('\r', r'\r');

DesktopEntry? currentDesktopEntry() => Platform.isLinux
    ? DesktopEntry.fromEnvironment(
        Platform.environment,
        dataHome: () => xdg.dataHome.path,
      )
    : null;

Future<void> refreshDesktopEntry() async {
  try {
    await currentDesktopEntry()?.refresh();
  } on Exception catch (error) {
    stderr.writeln('Could not update the applications menu entry: $error');
  }
}
