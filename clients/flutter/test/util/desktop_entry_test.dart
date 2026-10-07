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
    test('points the launcher at the AppImage', () {
      expect(
        desktopEntryText('/opt/apps/shadowmask.AppImage'),
        '[Desktop Entry]\n'
        'Type=Application\n'
        'Name=Shadowmask\n'
        'Exec="/opt/apps/shadowmask.AppImage"\n'
        'TryExec=/opt/apps/shadowmask.AppImage\n'
        'Icon=io.github.sndnv.shadowmask\n'
        'Categories=AudioVideo;Video;\n'
        'StartupWMClass=io.github.sndnv.shadowmask\n'
        'Terminal=false\n',
      );
    });

    test('escapes a path the launcher would otherwise misread', () {
      final List<String> lines = desktopEntryText(
        '/opt/my apps/sh"a\$d`ow\\m%k.AppImage',
      ).split('\n');

      expect(
        lines,
        contains(r'Exec="/opt/my apps/sh\\"a\\$d\\`ow\\\\m%%k.AppImage"'),
      );
      expect(lines, contains(r'TryExec=/opt/my apps/sh"a$d`ow\\m%k.AppImage'));
    });

    test('keeps a path with a line break on one line', () {
      final String text = desktopEntryText('/opt/a\nb.AppImage');

      expect(text.split('\n'), contains(r'Exec="/opt/a\nb.AppImage"'));
      expect(text.split('\n'), contains(r'TryExec=/opt/a\nb.AppImage'));
    });
  });

  test('adding writes the launcher entry and every icon size', () async {
    final DesktopEntry entry = entryFor('/opt/apps/shadowmask.AppImage');

    expect(await entry.installed(), isFalse);

    await entry.add();

    expect(await entry.installed(), isTrue);
    expect(
      entry.entryFile.path,
      p.join(dataHome, 'applications', 'io.github.sndnv.shadowmask.desktop'),
    );
    expect(
      await entry.entryFile.readAsString(),
      desktopEntryText('/opt/apps/shadowmask.AppImage'),
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

  test('removing deletes the entry and the icons', () async {
    final DesktopEntry entry = entryFor('/opt/apps/shadowmask.AppImage');
    await entry.add();

    await entry.remove();

    expect(await entry.installed(), isFalse);
    for (final int size in kDesktopIconSizes) {
      expect(await entry.iconFile(size).exists(), isFalse);
    }
  });

  test('removing what was never added succeeds', () async {
    await entryFor('/opt/apps/shadowmask.AppImage').remove();
  });

  test('adding fails when the mount has no icons', () async {
    await Directory(appDir).delete(recursive: true);

    await expectLater(
      entryFor('/opt/apps/shadowmask.AppImage').add(),
      throwsA(isA<FileSystemException>()),
    );
  });

  group('refresh', () {
    test('points an added entry at a moved AppImage', () async {
      await entryFor('/old/shadowmask.AppImage').add();
      final DesktopEntry moved = entryFor('/new/shadowmask.AppImage');

      await moved.refresh();

      expect(
        await moved.entryFile.readAsString(),
        desktopEntryText('/new/shadowmask.AppImage'),
      );
    });

    test('leaves an entry that already points here alone', () async {
      final DesktopEntry entry = entryFor('/opt/apps/shadowmask.AppImage');
      await entry.add();
      await entry.iconFile(64).delete();

      await entry.refresh();

      expect(await entry.iconFile(64).exists(), isFalse);
    });

    test('adds nothing when the entry was never added', () async {
      final DesktopEntry entry = entryFor('/opt/apps/shadowmask.AppImage');

      await entry.refresh();

      expect(await entry.installed(), isFalse);
      expect(await entry.iconFile(64).exists(), isFalse);
    });
  });
}
