import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/player/player_frame.dart';
import 'package:shadowmask/components/player/player_sheet.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/player_shortcuts.dart';

MouseCursor _videoCursor(WidgetTester tester) => tester
    .widgetList<MouseRegion>(find.byType(MouseRegion))
    .firstWhere((MouseRegion r) => r.onHover != null)
    .cursor;

double _chromeOpacity(WidgetTester tester) => tester
    .widget<AnimatedOpacity>(
      find
          .ancestor(
            of: find.text('back'),
            matching: find.byType(AnimatedOpacity),
          )
          .first,
    )
    .opacity;

Future<void> _tapPicture(WidgetTester tester) async {
  await tester.tapAt(tester.getCenter(find.byType(AspectRatio)));
  await tester.pump(kDoubleTapTimeout);
}

Future<void> _pump(
  WidgetTester tester, {
  required bool fullscreen,
  required Widget view,
  Size box = const Size(1600, 900),
  Widget? settings,
  VoidCallback? onDismissSettings,
  Widget? sheet,
  GlobalKey<ScaffoldState>? sheetHost,
  ValueChanged<bool>? onSheetChanged,
  bool touch = false,
  VoidCallback? onTapVideo,
  VoidCallback? onDoubleTapVideo,
  ValueChanged<int>? onSeekRelative,
  ValueChanged<double?>? onHoldSpeed,
  EdgeInsets safeArea = EdgeInsets.zero,
  String? waiting,
  VoidCallback? onGoBack,
  VoidCallback? onKeepWaiting,
  String keepWaitingLabel = Strings.playerKeepWaiting,
  Widget? diagnostics,
  double? videoAspect,
  bool playing = true,
  bool keepVisible = false,
  Widget overlay = const SizedBox(key: Key('overlay'), height: 40),
}) async {
  tester.view.physicalSize = box;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: MediaQuery(
          data: MediaQueryData(size: box, padding: safeArea),
          child: SizedBox(
            width: box.width,
            height: box.height,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Flexible(
                  fit: fullscreen ? FlexFit.tight : FlexFit.loose,
                  child: PlayerFrame(
                    view: view,
                    overlay: overlay,
                    back: const Text('back'),
                    playing: playing,
                    keepVisible: keepVisible,
                    fullscreen: fullscreen,
                    onTapVideo: onTapVideo ?? () {},
                    onDoubleTapVideo: onDoubleTapVideo,
                    onSeekRelative: onSeekRelative,
                    onHoldSpeed: onHoldSpeed,
                    touch: touch,
                    settings: settings,
                    onDismissSettings: onDismissSettings,
                    sheet: sheet,
                    sheetHost: sheetHost,
                    onSheetChanged: onSheetChanged,
                    waiting: waiting,
                    onGoBack: onGoBack,
                    onKeepWaiting: onKeepWaiting,
                    keepWaitingLabel: keepWaitingLabel,
                    diagnostics: diagnostics,
                    videoAspect: videoAspect,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pump();
}

Future<void> _doubleTapAt(WidgetTester tester, Offset at) async {
  await tester.tapAt(at);
  await tester.pump(kDoubleTapMinTime);
  await tester.tapAt(at);
  await tester.pump(kDoubleTapTimeout);
}

double _holdBadgeOpacity(WidgetTester tester, double rate) => tester
    .widget<AnimatedOpacity>(
      find
          .ancestor(
            of: find.text(Strings.playerHoldSpeed(rate)),
            matching: find.byType(AnimatedOpacity),
          )
          .first,
    )
    .opacity;

double _badgeOpacity(WidgetTester tester, IconData icon) => tester
    .widget<AnimatedOpacity>(
      find
          .ancestor(
            of: find.byIcon(icon),
            matching: find.byType(AnimatedOpacity),
          )
          .first,
    )
    .opacity;

void main() {
  testWidgets('a windowed frame is no taller than the picture it holds', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      fullscreen: false,
      view: const _View(key: Key('view')),
      box: const Size(1600, 1200),
    );

    expect(
      tester.getSize(find.byType(PlayerFrame)).height,
      tester.getSize(find.byKey(const Key('view'))).height,
      reason: 'any excess is painted as bars above and below the picture',
    );
  });

  testWidgets('a wider film makes a shorter frame, not black bars', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      fullscreen: false,
      view: const _View(key: Key('view')),
      box: const Size(1600, 1200),
      videoAspect: 2.39,
    );

    final Size stage = tester.getSize(find.byKey(const Key('view')));
    expect(stage.width, 1600);
    expect(stage.width / stage.height, closeTo(2.39, 0.01));
    expect(tester.getSize(find.byType(PlayerFrame)).height, stage.height);
  });

  testWidgets('fullscreen still fills the screen it was given', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(key: Key('view')),
      box: const Size(1600, 1200),
      videoAspect: 2.39,
    );

    expect(
      tester.getSize(find.byType(PlayerFrame)),
      const Size(1600, 1200),
      reason: 'fullscreen means the whole screen, letterboxing and all',
    );
  });

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

  testWidgets('tapping the video dismisses a panel and reaches nothing else', (
    WidgetTester tester,
  ) async {
    int dismissed = 0;
    int taps = 0;
    await _pump(
      tester,
      fullscreen: false,
      view: const _View(),
      box: const Size(360, 800),
      settings: const SizedBox(key: Key('panel'), width: 340, height: 400),
      onDismissSettings: () => dismissed++,
      onTapVideo: () => taps++,
    );

    await tester.tapAt(
      tester.getRect(find.byType(AspectRatio)).topLeft + const Offset(4, 4),
    );
    await tester.pump(kDoubleTapTimeout);

    expect(dismissed, 1);
    expect(taps, 0);
  });

  testWidgets('a tap in a dialog over the player keeps the panel open', (
    WidgetTester tester,
  ) async {
    int dismissed = 0;
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      settings: Builder(
        builder: (BuildContext context) => TextButton(
          onPressed: () => showDialog<void>(
            context: context,
            builder: (BuildContext _) =>
                const AlertDialog(content: Text('help body')),
          ),
          child: const Text('open help'),
        ),
      ),
      onDismissSettings: () => dismissed++,
    );

    await tester.tap(find.text('open help'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('help body'));
    await tester.pump();

    expect(dismissed, 0);
  });

  testWidgets('the controls hide on their own once a panel lets them go', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      touch: true,
      keepVisible: true,
    );
    await tester.pump(const Duration(seconds: 3));
    expect(_chromeOpacity(tester), 1);

    await _pump(tester, fullscreen: true, view: const _View(), touch: true);
    await tester.pump(const Duration(seconds: 3));

    expect(_chromeOpacity(tester), 0);
  });

  testWidgets('the bar keeps its state when the waiting layer comes and goes', (
    WidgetTester tester,
  ) async {
    const Widget bar = _View(key: Key('bar'));
    await _pump(tester, fullscreen: true, view: const _View(), overlay: bar);
    final _ViewState before = tester.state<_ViewState>(
      find.byKey(const Key('bar')),
    );

    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      overlay: bar,
      waiting: 'loading',
    );
    await _pump(tester, fullscreen: true, view: const _View(), overlay: bar);

    expect(
      identical(tester.state<_ViewState>(find.byKey(const Key('bar'))), before),
      isTrue,
    );
    expect(before.mounts, 1);
  });

  testWidgets('a tap beside the sheet closes it and reaches nothing else', (
    WidgetTester tester,
  ) async {
    final GlobalKey<ScaffoldState> host = GlobalKey<ScaffoldState>();
    final List<bool> changes = <bool>[];
    int taps = 0;
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      box: const Size(900, 400),
      touch: true,
      sheet: const SizedBox(key: Key('sheet')),
      sheetHost: host,
      onSheetChanged: changes.add,
      onTapVideo: () => taps++,
    );

    host.currentState!.openEndDrawer();
    await tester.pumpAndSettle();
    final Rect sheet = tester.getRect(find.byKey(const Key('sheet')));
    expect(sheet.right, 900);
    expect(sheet.width, playerSheetWidth(900));

    await tester.tapAt(const Offset(100, 200));
    await tester.pumpAndSettle();
    await tester.pump(kDoubleTapTimeout);

    expect(find.byKey(const Key('sheet')), findsNothing);
    expect(changes, <bool>[true, false]);
    expect(taps, 0);
    expect(_chromeOpacity(tester), 1);
  });

  testWidgets('on touch a tap hides the controls that show, and shows them', (
    WidgetTester tester,
  ) async {
    await _pump(tester, fullscreen: true, view: const _View(), touch: true);
    expect(_chromeOpacity(tester), 1);

    await _tapPicture(tester);

    expect(_chromeOpacity(tester), 0);

    await _tapPicture(tester);

    expect(_chromeOpacity(tester), 1);
  });

  testWidgets('a paused touch player hides the controls on a tap too', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      touch: true,
      playing: false,
    );
    await tester.pump(const Duration(seconds: 3));
    expect(_chromeOpacity(tester), 1);

    await _tapPicture(tester);
    await tester.pump(const Duration(seconds: 3));

    expect(_chromeOpacity(tester), 0);
  });

  testWidgets('a pointer tap on the picture only shows the controls', (
    WidgetTester tester,
  ) async {
    await _pump(tester, fullscreen: true, view: const _View());

    await _tapPicture(tester);

    expect(_chromeOpacity(tester), 1);
  });

  testWidgets('a touch tap while a panel holds the controls leaves them up', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      touch: true,
      keepVisible: true,
    );

    await _tapPicture(tester);

    expect(_chromeOpacity(tester), 1);
  });

  testWidgets('a tap on a control in the bar dismisses an open panel', (
    WidgetTester tester,
  ) async {
    int dismissed = 0;
    int pressed = 0;
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      settings: const SizedBox(key: Key('panel'), width: 300, height: 200),
      onDismissSettings: () => dismissed++,
      overlay: TextButton(
        onPressed: () => pressed++,
        child: const Text('bar control'),
      ),
    );

    await tester.tap(find.text('bar control'));
    await tester.pump();

    expect(dismissed, 1);
    expect(pressed, 1);
  });

  testWidgets('a touch double tap seeks by the half of the frame it lands on', (
    WidgetTester tester,
  ) async {
    final List<int> seeks = <int>[];
    int fullscreens = 0;
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      box: const Size(800, 360),
      touch: true,
      onSeekRelative: seeks.add,
      onDoubleTapVideo: () => fullscreens++,
    );

    final Rect stage = tester.getRect(find.byType(AspectRatio));
    await _doubleTapAt(tester, stage.centerLeft + const Offset(40, 0));
    await _doubleTapAt(tester, stage.centerRight - const Offset(40, 0));

    expect(seeks, <int>[-10000, 10000]);
    expect(
      fullscreens,
      0,
      reason: 'double tap is the seek gesture on touch, not fullscreen',
    );
  });

  testWidgets('a mouse double tap still toggles fullscreen', (
    WidgetTester tester,
  ) async {
    final List<int> seeks = <int>[];
    int fullscreens = 0;
    await _pump(
      tester,
      fullscreen: false,
      view: const _View(),
      onSeekRelative: seeks.add,
      onDoubleTapVideo: () => fullscreens++,
    );

    await _doubleTapAt(tester, tester.getRect(find.byType(AspectRatio)).center);

    expect(fullscreens, 1);
    expect(seeks, isEmpty);
  });

  testWidgets('a seek badge appears on the seeked side, then clears', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      box: const Size(800, 360),
      touch: true,
      onSeekRelative: (_) {},
    );

    expect(_badgeOpacity(tester, Icons.forward_10), 0);

    final Rect stage = tester.getRect(find.byType(AspectRatio));
    await _doubleTapAt(tester, stage.centerLeft + const Offset(40, 0));
    await tester.pumpAndSettle();

    expect(_badgeOpacity(tester, Icons.replay_10), 1);
    expect(find.byIcon(Icons.forward_10), findsNothing);

    await tester.pumpAndSettle(const Duration(seconds: 1));

    expect(_badgeOpacity(tester, Icons.replay_10), 0);
  });

  testWidgets('the chrome clears the notch and the home indicator', (
    WidgetTester tester,
  ) async {
    const EdgeInsets safe = EdgeInsets.fromLTRB(48, 0, 0, 24);
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      box: const Size(800, 360),
      touch: true,
      safeArea: safe,
    );

    final Rect stage = tester.getRect(find.byType(AspectRatio));
    final Rect back = tester.getRect(find.text('back'));
    final Rect overlay = tester.getRect(find.byKey(const Key('overlay')));

    expect(back.left, greaterThanOrEqualTo(stage.left + safe.left));
    // The bar reaches the edge so its scrim does too; clearing the home
    // indicator is the bar's own padding, asserted in its own test.
    expect(overlay.bottom, closeTo(stage.bottom, 0.5));
  });

  testWidgets('holding climbs the rates and releasing puts it back', (
    WidgetTester tester,
  ) async {
    final List<double?> held = <double?>[];
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      box: const Size(800, 360),
      touch: true,
      onHoldSpeed: held.add,
    );

    final Offset middle = tester.getRect(find.byType(AspectRatio)).center;
    final TestGesture gesture = await tester.startGesture(middle);
    await tester.pump(kLongPressTimeout + const Duration(milliseconds: 50));

    // The first rung is 3x rather than 2x so that a viewer already at 2x from
    // the settings panel feels the hold do something.
    expect(held, <double?>[3]);
    expect(_holdBadgeOpacity(tester, 3), 1);

    await tester.pump(kHoldStep);
    expect(held, <double?>[3, 5]);
    expect(_holdBadgeOpacity(tester, 5), 1);

    await tester.pump(kHoldStep);
    expect(held, <double?>[3, 5, 10]);

    // The last rate is the ceiling; holding longer must not keep climbing.
    await tester.pump(kHoldStep * 3);
    expect(held, <double?>[3, 5, 10]);

    await gesture.up();
    await tester.pumpAndSettle();

    expect(held, <double?>[3, 5, 10, null]);
    expect(_holdBadgeOpacity(tester, 10), 0);
  });

  testWidgets('a pointer client never holds to speed up', (
    WidgetTester tester,
  ) async {
    // The gesture belongs to touch; a mouse has the settings panel and the
    // keyboard shortcuts instead.
    final List<double?> held = <double?>[];
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      box: const Size(800, 360),
      onHoldSpeed: held.add,
    );

    final Offset middle = tester.getRect(find.byType(AspectRatio)).center;
    final TestGesture gesture = await tester.startGesture(middle);
    await tester.pump(kLongPressTimeout + const Duration(milliseconds: 50));
    await gesture.up();
    await tester.pumpAndSettle();

    expect(held, isEmpty);
  });

  testWidgets('a hold that is cancelled still puts the rate back', (
    WidgetTester tester,
  ) async {
    final List<double?> held = <double?>[];
    await _pump(
      tester,
      fullscreen: true,
      view: const _View(),
      box: const Size(800, 360),
      touch: true,
      onHoldSpeed: held.add,
    );

    final Offset middle = tester.getRect(find.byType(AspectRatio)).center;
    final TestGesture gesture = await tester.startGesture(middle);
    await tester.pump(kLongPressTimeout + const Duration(milliseconds: 50));
    await gesture.cancel();
    await tester.pumpAndSettle();

    expect(held.last, isNull, reason: 'the rate must not stay raised');
  });

  testWidgets('the waiting controls stay reachable behind the diagnostics', (
    WidgetTester tester,
  ) async {
    // The readout sits in the top right corner and a phone in landscape is
    // short, so a diagnostics box tall enough to reach the centred buttons
    // used to swallow their taps.
    int backs = 0;
    await _pump(
      tester,
      fullscreen: false,
      view: const _View(),
      box: const Size(800, 360),
      waiting: 'still working',
      onGoBack: () => backs++,
      onKeepWaiting: () {},
      diagnostics: const SizedBox(width: 400, height: 360),
    );

    await tester.tap(find.text(Strings.playerGoBack));
    await tester.pump();

    expect(backs, 1);
  });

  testWidgets('a failure offers to dismiss it instead of to keep waiting', (
    WidgetTester tester,
  ) async {
    int dismissed = 0;
    await _pump(
      tester,
      fullscreen: false,
      view: const _View(),
      waiting: Strings.playerFailed,
      onGoBack: () {},
      onKeepWaiting: () => dismissed++,
      keepWaitingLabel: Strings.dismiss,
    );

    expect(find.text(Strings.playerKeepWaiting), findsNothing);
    await tester.tap(find.text(Strings.dismiss));
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
