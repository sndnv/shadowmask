import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:path/path.dart' as p;
import 'package:shadowmask/util/desktop_entry_native.dart';

void main() {
  late Directory root;
  late String appDir;
  late String dataHome;

  setUp(() async {
    root = await Directory.systemTemp.createTemp('desktop_entry_test');
    appDir = p.join(root.path, 'mount');
    dataHome = p.join(root.path, 'data');
    for (final int size in kDesktopIconSizes) {
      final File icon = File(
        p.join(
          appDir,
          'usr',
          'share',
          'icons',
          'hicolor',
          '${size}x$size',
          'apps',
          'shadowmask.png',
        ),
      );
      await icon.parent.create(recursive: true);
      await icon.writeAsString('icon $size');
    }
  });

  tearDown(() => root.delete(recursive: true));

  DesktopEntry entryFor(String appImage) =>
      DesktopEntry(appImage: appImage, appDir: appDir, dataHome: dataHome);

  Future<String> download(String version) async {
    final File file = File(
      p.join(
        root.path,
        'downloads',
        'shadowmask-client-linux-$version.AppImage',
      ),
    );
    await file.parent.create(recursive: true);
    await file.writeAsString('AppImage $version');
    await Process.run('chmod', <String>['755', file.path]);
    return file.path;
  }

  group('fromEnvironment', () {
    test('needs both the AppImage path and its mount', () {
      int reads = 0;
      String dataHomeCalled() {
        reads++;
        return '/home/u/.local/share';
      }

      expect(
        DesktopEntry.fromEnvironment(<String, String>{
          'APPDIR': '/mnt',
        }, dataHome: dataHomeCalled),
        isNull,
      );
      expect(
        DesktopEntry.fromEnvironment(<String, String>{
          'APPIMAGE': '/a.AppImage',
        }, dataHome: dataHomeCalled),
        isNull,
      );
      expect(
        DesktopEntry.fromEnvironment(<String, String>{
          'APPIMAGE': '',
          'APPDIR': '/mnt',
        }, dataHome: dataHomeCalled),
        isNull,
      );
      expect(reads, 0);
    });

    test('no data home means no entry', () {
      expect(
        DesktopEntry.fromEnvironment(<String, String>{
          'APPIMAGE': '/a.AppImage',
          'APPDIR': '/mnt',
        }, dataHome: () => throw StateError('HOME is not set')),
        isNull,
      );
    });

    test('an AppImage run carries its paths', () {
      final DesktopEntry? entry = DesktopEntry.fromEnvironment(<String, String>{
        'APPIMAGE': '/a.AppImage',
        'APPDIR': '/mnt',
      }, dataHome: () => '/home/u/.local/share');

      expect(entry?.appImage, '/a.AppImage');
      expect(entry?.appDir, '/mnt');
      expect(entry?.dataHome, '/home/u/.local/share');
    });
  });

  group('desktopEntryText', () {
    const String icon =
        '/home/u/.local/share/icons/hicolor/512x512/apps/io.github.sndnv.shadowmask.png';

    test('points the launcher at the AppImage and its icon file', () {
      expect(
        desktopEntryText('/opt/apps/shadowmask.AppImage', icon),
        '[Desktop Entry]\n'
        'Type=Application\n'
        'Name=Shadowmask\n'
        'Exec="/opt/apps/shadowmask.AppImage"\n'
        'TryExec=/opt/apps/shadowmask.AppImage\n'
        'Icon=$icon\n'
        'Categories=AudioVideo;Video;\n'
        'StartupWMClass=io.github.sndnv.shadowmask\n'
        'Terminal=false\n',
      );
    });

    test('escapes a path the launcher would otherwise misread', () {
      final List<String> lines = desktopEntryText(
        '/opt/my apps/sh"a\$d`ow\\m%k.AppImage',
        icon,
      ).split('\n');

      expect(
        lines,
        contains(r'Exec="/opt/my apps/sh\\"a\\$d\\`ow\\\\m%%k.AppImage"'),
      );
      expect(lines, contains(r'TryExec=/opt/my apps/sh"a$d`ow\\m%k.AppImage'));
    });

    test('keeps a path with a line break on one line', () {
      final String text = desktopEntryText(
        '/opt/a\nb.AppImage',
        '/home/a\nb/icon.png',
      );

      expect(text.split('\n'), contains(r'Exec="/opt/a\nb.AppImage"'));
      expect(text.split('\n'), contains(r'TryExec=/opt/a\nb.AppImage'));
      expect(text.split('\n'), contains(r'Icon=/home/a\nb/icon.png'));
    });
  });

  test('adding copies the AppImage, then writes the launcher entry for the '
      'copy and every icon size', () async {
    final String downloaded = await download('v0.0.7');
    final DesktopEntry entry = entryFor(downloaded);

    expect(await entry.installed(), isFalse);

    await entry.add();

    expect(await entry.installed(), isTrue);
    final File copy = entry.installedAppImage;
    expect(copy.path, p.join(dataHome, 'shadowmask', 'shadowmask.AppImage'));
    expect(await copy.readAsString(), 'AppImage v0.0.7');
    expect((await copy.stat()).modeString(), 'rwxr-xr-x');
    expect(await File('${copy.path}.part').exists(), isFalse);
    expect(
      entry.entryFile.path,
      p.join(dataHome, 'applications', 'io.github.sndnv.shadowmask.desktop'),
    );
    expect(await entry.entryFile.readAsString(), entry.entryText);
    expect(
      (await entry.entryFile.readAsString()).split('\n'),
      containsAll(<String>['Exec="${copy.path}"', 'TryExec=${copy.path}']),
      reason: 'the download can be deleted or replaced by the next release',
    );
    expect(
      (await entry.entryFile.readAsString()).split('\n'),
      contains('Icon=${entry.iconFile(512).path}'),
      reason:
          'a name would be looked up through icon caches other programs own',
    );
    for (final int size in kDesktopIconSizes) {
      expect(
        entry.iconFile(size).path,
        p.join(
          dataHome,
          'icons',
          'hicolor',
          '${size}x$size',
          'apps',
          'io.github.sndnv.shadowmask.png',
        ),
      );
      expect(await entry.iconFile(size).readAsString(), 'icon $size');
    }
  });

  test('removing deletes the entry, the copy and the icons', () async {
    final DesktopEntry entry = entryFor(await download('v0.0.7'));
    await entry.add();

    await entry.remove();

    expect(await entry.installed(), isFalse);
    expect(await entry.installedAppImage.exists(), isFalse);
    for (final int size in kDesktopIconSizes) {
      expect(await entry.iconFile(size).exists(), isFalse);
    }
  });

  test('removing what was never added succeeds', () async {
    await entryFor(await download('v0.0.7')).remove();
  });

  test('adding fails when the mount has no icons', () async {
    await Directory(appDir).delete(recursive: true);

    await expectLater(
      entryFor(await download('v0.0.7')).add(),
      throwsA(isA<FileSystemException>()),
    );
  });

  group('refresh', () {
    Future<DesktopEntry> fromTheMenu() async {
      final DesktopEntry added = entryFor(await download('v0.0.7'));
      await added.add();
      await File(added.appImage).delete();
      return entryFor(added.installedAppImage.path);
    }

    test('a downloaded AppImage started by hand replaces the copy', () async {
      await entryFor(await download('v0.0.7')).add();
      final DesktopEntry newer = entryFor(await download('v0.0.8'));

      await newer.refresh();

      expect(await newer.installedAppImage.readAsString(), 'AppImage v0.0.8');
      expect(await newer.entryFile.readAsString(), newer.entryText);
    });

    test('an entry from before the copy is moved over to it', () async {
      final DesktopEntry entry = entryFor(await download('v0.0.6'));
      await entry.entryFile.parent.create(recursive: true);
      await entry.entryFile.writeAsString(
        desktopEntryText(entry.appImage, entry.iconFile(512).path),
      );

      await entry.refresh();

      expect(await entry.installedAppImage.readAsString(), 'AppImage v0.0.6');
      expect(await entry.entryFile.readAsString(), entry.entryText);
    });

    test('starting from the menu copies nothing', () async {
      final DesktopEntry entry = await fromTheMenu();
      await entry.iconFile(64).delete();

      await entry.refresh();

      expect(await entry.iconFile(64).exists(), isFalse);
      expect(await entry.installedAppImage.readAsString(), 'AppImage v0.0.7');
    });

    test('a link to the copy counts as the copy', () async {
      final DesktopEntry copy = await fromTheMenu();
      final Link link = Link(p.join(root.path, 'shadowmask.AppImage'));
      await link.create(copy.installedAppImage.path);
      await copy.iconFile(64).delete();

      await entryFor(link.path).refresh();

      expect(await copy.iconFile(64).exists(), isFalse);
    });

    test('points an entry that names its icon at the icon file', () async {
      final DesktopEntry entry = await fromTheMenu();
      await entry.entryFile.writeAsString(
        entry.entryText.replaceFirst(
          RegExp(r'^Icon=.*$', multiLine: true),
          'Icon=io.github.sndnv.shadowmask',
        ),
      );

      await entry.refresh();

      expect(await entry.entryFile.readAsString(), entry.entryText);
    });

    test('adds nothing when the entry was never added', () async {
      final DesktopEntry entry = entryFor(await download('v0.0.7'));

      await entry.refresh();

      expect(await entry.installed(), isFalse);
      expect(await entry.installedAppImage.exists(), isFalse);
      expect(await entry.iconFile(64).exists(), isFalse);
    });
  });
}
