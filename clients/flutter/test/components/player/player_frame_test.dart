import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/player/player_frame.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

MouseCursor _videoCursor(WidgetTester tester) => tester
    .widgetList<MouseRegion>(find.byType(MouseRegion))
    .firstWhere((MouseRegion r) => r.onHover != null)
    .cursor;

Future<void> _pump(
  WidgetTester tester, {
  required bool fullscreen,
  required Widget view,
  Size box = const Size(1600, 900),
  Widget? settings,
  VoidCallback? onDismissSettings,
}) async {
  tester.view.physicalSize = box;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: SizedBox(
          width: box.width,
          height: box.height,
          child: PlayerFrame(
            view: view,
            overlay: const SizedBox(height: 40),
            back: const Text('back'),
            playing: true,
            fullscreen: fullscreen,
            onTapVideo: () {},
            settings: settings,
            onDismissSettings: onDismissSettings,
          ),
        ),
      ),
    ),
  );
  await tester.pump();
}

void main() {
  testWidgets('the view is not rebuilt when fullscreen toggles', (
    WidgetTester tester,
  ) async {
    const Widget view = _View(key: Key('view'));

    await _pump(tester, fullscreen: false, view: view);
    final _ViewState before = tester.state<_ViewState>(
      find.byKey(const Key('view')),
    );
    expect(before.mounts, 1);

    await _pump(tester, fullscreen: true, view: view);

    expect(
      identical(
        tester.state<_ViewState>(find.byKey(const Key('view'))),
        before,
      ),
      isTrue,
    );
    expect(before.mounts, 1);

    await _pump(tester, fullscreen: false, view: view);

    expect(
      identical(
        tester.state<_ViewState>(find.byKey(const Key('view'))),
        before,
      ),
      isTrue,
    );
  });

  testWidgets('fullscreen fills the box and windowed takes the full width', (
    WidgetTester tester,
  ) async {
    const Widget view = _View(key: Key('view'));

    await _pump(tester, fullscreen: true, view: view);
    expect(
      tester.getSize(find.byKey(const Key('view'))),
      const Size(1600, 900),
    );

    await _pump(
      tester,
      fullscreen: false,
      view: view,
      box: const Size(1600, 1200),
    );
    final Size windowed = tester.getSize(find.byKey(const Key('view')));
    expect(windowed.width, 1600);
    expect(windowed.width / windowed.height, closeTo(16 / 9, 0.01));
  });

  testWidgets('a box wider than 16:9 binds the windowed stage on height', (
    WidgetTester tester,
  ) async {
    const Widget view = _View(key: Key('view'));

    await _pump(
      tester,
      fullscreen: false,
      view: view,
      box: const Size(1600, 600),
    );

    final Size windowed = tester.getSize(find.byKey(const Key('view')));
    expect(windowed.height, 600);
    expect(windowed.width / windowed.height, closeTo(16 / 9, 0.01));
  });

  testWidgets('fullscreen hides the cursor once the overlay fades', (
    WidgetTester tester,
  ) async {
    await _pump(tester, fullscreen: true, view: const _View());

    expect(_videoCursor(tester), MouseCursor.defer);

    await tester.pump(const Duration(seconds: 3));

    expect(_videoCursor(tester), SystemMouseCursors.none);
  });

  testWidgets('a fading overlay leaves the cursor alone when windowed', (
    WidgetTester tester,
  ) async {
    await _pump(tester, fullscreen: false, view: const _View());

    await tester.pump(const Duration(seconds: 3));

    expect(_videoCursor(tester), MouseCursor.defer);
  });

  testWidgets('a settings panel stays inside the stage on a phone', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      fullscreen: false,
      view: const _View(),
      box: const Size(360, 800),
      settings: const SizedBox(
        key: Key('panel'),
        width: 340,
        height: 400,
        child: ColoredBox(color: Colors.white),
      ),
    );

    final Rect stage = tester.getRect(find.byType(AspectRatio));
    final Rect panel = tester.getRect(find.byKey(const Key('panel')));

    expect(
      stage.contains(panel.topLeft) && stage.contains(panel.bottomRight),
      isTrue,
      reason:
          'a panel hanging off the stage is clipped, and its close button '
          'goes with it',
    );
  });

  testWidgets('tapping the video dismisses a panel that covers the stage', (
    WidgetTester tester,
  ) async {
    int dismissed = 0;
    await _pump(
      tester,
      fullscreen: false,
      view: const _View(),
      box: const Size(360, 800),
      settings: const SizedBox(key: Key('panel'), width: 340, height: 400),
      onDismissSettings: () => dismissed++,
    );

    await tester.tapAt(
      tester.getRect(find.byType(AspectRatio)).topLeft + const Offset(4, 4),
    );
    await tester.pump();

    expect(dismissed, 1);
  });
}

class _View extends StatefulWidget {
  const _View({super.key});

  @override
  State<_View> createState() => _ViewState();
}

class _ViewState extends State<_View> {
  int mounts = 0;

  @override
  void initState() {
    super.initState();
    mounts++;
  }

  @override
  Widget build(BuildContext context) => const ColoredBox(color: Colors.black);
}
