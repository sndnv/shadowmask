import 'package:material_ui/material_ui.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/linked_nodes.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/tokens.dart';

const Tokens _t = Tokens.hologram;
final Shapes _shapes = Shapes.cut(_t, highContrast: false);

NodeItem _item(
  String label, {
  bool selected = false,
  VoidCallback? onTap,
  String? value,
  Color? selectedColor,
  NodeRole role = NodeRole.choice,
}) => NodeItem(
  label: label,
  selected: selected,
  onTap: onTap ?? () {},
  value: value,
  selectedColor: selectedColor,
  role: role,
);

Future<void> _pump(
  WidgetTester tester,
  Widget child, {
  double width = 400,
  ValueChanged<LogicalKeyboardKey>? onOutsideKey,
}) => tester.pumpWidget(
  MaterialApp(
    theme: buildTheme(AppThemeVariant.hologram),
    home: Scaffold(
      body: Focus(
        onKeyEvent: (FocusNode node, KeyEvent event) {
          if (event is KeyUpEvent || onOutsideKey == null) {
            return KeyEventResult.ignored;
          }
          onOutsideKey(event.logicalKey);
          return KeyEventResult.handled;
        },
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(width: width, child: child),
        ),
      ),
    ),
  ),
);

LinkedNodes _map({
  int from = 0,
  int lit = 1,
  bool rightEnabled = true,
  VoidCallback? onRight,
  VoidCallback? onLeft,
  List<String> right = const <String>['W', 'X', 'Y', 'Z'],
}) => LinkedNodes(
  from: from,
  lit: lit,
  rightLabel: 'Picks',
  rightEnabled: rightEnabled,
  left: <Widget>[
    _item('A', selected: from == 0, onTap: onLeft, role: NodeRole.opens),
    _item('B', selected: from == 1, role: NodeRole.opens),
    _item('C', selected: from == 2, role: NodeRole.opens),
  ],
  right: <Widget>[
    for (int i = 0; i < right.length; i++)
      _item(right[i], selected: i == lit, onTap: onRight),
  ],
);

Finder get _links => find.byWidgetPredicate(
  (Widget w) => w is CustomPaint && w.painter is LinkPainter,
);

Finder _node(String label) =>
    find.ancestor(of: find.text(label), matching: find.byType(NodeItem));

Material _material(WidgetTester tester, String label) =>
    tester.widget<Material>(
      find.descendant(of: _node(label), matching: find.byType(Material)).first,
    );

String? _focused() => FocusManager.instance.primaryFocus?.context
    ?.findAncestorWidgetOfExactType<NodeItem>()
    ?.label;

Offset _dotAt(int row) =>
    Offset(kLinkWidth - Marks.readoutDot.width / 2, nodeCentre(row));

Offset _sourceAt(int row) => Offset(kLinkSource.width / 2, nodeCentre(row));

Future<void> _press(WidgetTester tester, LogicalKeyboardKey key) async {
  await tester.sendKeyEvent(key);
  await tester.pump();
}

Future<void> _focus(WidgetTester tester, String label) async {
  Focus.of(tester.element(find.text(label))).requestFocus();
  await tester.pump();
}

void main() {
  testWidgets('rows sit 36 tall and 8 apart and the links span the taller '
      'column', (WidgetTester tester) async {
    await _pump(tester, _map());

    expect(tester.getSize(_node('W')).height, kNodeHeight);
    expect(
      tester.getTopLeft(_node('Y')).dy - tester.getTopLeft(_node('W')).dy,
      2 * (kNodeHeight + kNodeGap),
    );
    expect(tester.getTopLeft(_node('C')).dy, tester.getTopLeft(_node('Y')).dy);
    expect(
      tester.getSize(_links),
      const Size(kLinkWidth, 4 * kNodeHeight + 3 * kNodeGap),
    );
  });

  testWidgets('every choice is linked to the open node and the current one '
      'is bright and drawn last', (WidgetTester tester) async {
    await _pump(tester, _map());
    final Color soft = _shapes.lineSoft(_t.border);
    final Color dot = _shapes.line(_t.border);

    expect(
      _links,
      paints
        ..path(color: soft)
        ..path(color: dot, includes: <Offset>[_dotAt(0)])
        ..path(color: soft)
        ..path(color: dot, includes: <Offset>[_dotAt(2)])
        ..path(color: soft)
        ..path(color: dot, includes: <Offset>[_dotAt(3)])
        ..path(color: _t.accent)
        ..path(color: _t.accent, includes: <Offset>[_dotAt(1)])
        ..path(color: _t.accent, includes: <Offset>[_sourceAt(0)]),
    );
  });

  testWidgets('the links start at the open node', (WidgetTester tester) async {
    await _pump(tester, _map(from: 2, lit: 0));

    expect(
      _links,
      paints
        ..path()
        ..path(includes: <Offset>[_dotAt(1)])
        ..path()
        ..path(includes: <Offset>[_dotAt(2)])
        ..path()
        ..path(includes: <Offset>[_dotAt(3)])
        ..path()
        ..path(includes: <Offset>[_dotAt(0)])
        ..path(
          includes: <Offset>[_sourceAt(2)],
          excludes: <Offset>[_sourceAt(0)],
        ),
    );
  });

  testWidgets('choices start level with the open node and move up only to '
      'stay inside the map', (WidgetTester tester) async {
    LinkedNodes tall(int from) => LinkedNodes(
      from: from,
      lit: 0,
      left: <Widget>[
        for (final String label in <String>['A', 'B', 'C', 'D', 'E', 'F'])
          _item(label, selected: label == 'ABCDEF'[from], role: NodeRole.opens),
      ],
      right: <Widget>[_item('on', selected: true), _item('off')],
    );

    await _pump(tester, tall(3));
    expect(tester.getCenter(_node('on')).dy, tester.getCenter(_node('D')).dy);
    expect(
      _links,
      paints
        ..path()
        ..path(includes: <Offset>[_dotAt(4)])
        ..path()
        ..path(includes: <Offset>[_dotAt(3)])
        ..path(includes: <Offset>[_sourceAt(3)]),
    );

    await _pump(tester, tall(5));
    expect(tester.getCenter(_node('on')).dy, tester.getCenter(_node('E')).dy);
    expect(tester.getCenter(_node('off')).dy, tester.getCenter(_node('F')).dy);
    expect(tester.getSize(_links).height, nodesHeight(6));

    expect(rightStart(from: 0, left: 3, right: 4), 0);
    expect(rightStart(from: 2, left: 3, right: 1), 2);
  });

  testWidgets('a node without a tap is a read-out: no button, no focus', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(
      tester,
      Semantics(
        container: true,
        child: const SizedBox(
          height: kNodeHeight,
          child: NodeItem(
            label: 'Mode',
            value: 'transcode',
            selected: false,
            onTap: null,
          ),
        ),
      ),
    );

    expect(
      tester.getSemantics(find.text('Mode')),
      isSemantics(isButton: false, isFocusable: false),
    );
    expect(
      find.descendant(
        of: find.byType(NodeItem),
        matching: find.byWidgetPredicate(
          (Widget w) => w is Focus && w.canRequestFocus == true,
        ),
      ),
      findsNothing,
    );
    semantics.dispose();
  });

  testWidgets('a plain node is a button with no selected or expanded state', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(
      tester,
      Semantics(
        container: true,
        child: SizedBox(
          height: kNodeHeight,
          child: _item('Log', role: NodeRole.plain),
        ),
      ),
    );

    expect(
      tester.getSemantics(find.text('Log')),
      isSemantics(
        label: 'Log',
        isButton: true,
        hasSelectedState: false,
        hasExpandedState: false,
      ),
    );
    semantics.dispose();
  });

  testWidgets('a disabled node dims, keeps its focus and does nothing', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    int taps = 0;
    Widget step({required bool enabled}) => Semantics(
      container: true,
      child: SizedBox(
        height: kNodeHeight,
        child: NodeItem(
          label: 'Later',
          selected: false,
          role: NodeRole.plain,
          enabled: enabled,
          onTap: () => taps++,
        ),
      ),
    );
    await _pump(tester, step(enabled: true));
    await _focus(tester, 'Later');

    await _pump(tester, step(enabled: false));
    expect(_focused(), 'Later');
    await _press(tester, LogicalKeyboardKey.enter);
    await tester.tap(_node('Later'));

    expect(taps, 0);
    expect(
      tester
          .widget<Opacity>(
            find.descendant(of: _node('Later'), matching: find.byType(Opacity)),
          )
          .opacity,
      0.5,
    );
    expect(
      tester.getSemantics(find.text('Later')),
      isSemantics(label: 'Later', isButton: true, isEnabled: false),
    );
    semantics.dispose();
  });

  testWidgets('a node can end in an icon instead of a value', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      SizedBox(
        height: kNodeHeight,
        child: NodeItem(
          label: 'Log',
          icon: Icons.open_in_new,
          selected: false,
          onTap: () {},
        ),
      ),
    );

    expect(
      tester.getCenter(find.byIcon(Icons.open_in_new)).dx,
      greaterThan(tester.getCenter(find.text('Log')).dx),
    );
    expect(tester.widget<Icon>(find.byIcon(Icons.open_in_new)).color, _t.muted);
  });

  testWidgets('a caption reads as a monospace capital heading', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(
      tester,
      const SizedBox(height: kNodeHeight, child: NodeCaption('Buffering')),
    );

    final Text caption = tester.widget<Text>(find.text('BUFFERING'));
    expect(caption.style?.fontFamily, kMonoFamily);
    expect(caption.style?.color, _t.muted);
    expect(
      tester.getSemantics(find.text('BUFFERING')),
      isSemantics(label: 'BUFFERING', isHeader: true),
    );
    semantics.dispose();
  });

  testWidgets('a node without choices draws no links', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _map(right: const <String>[]));

    expect(_links, paintsNothing);
    expect(tester.getSize(_links).height, nodesHeight(3));
  });

  testWidgets('blocked choices dim and take no taps while the nodes still do', (
    WidgetTester tester,
  ) async {
    int right = 0;
    int left = 0;
    await _pump(
      tester,
      _map(rightEnabled: false, onRight: () => right++, onLeft: () => left++),
    );

    await tester.tap(_node('W'), warnIfMissed: false);
    await tester.tap(_node('A'));

    expect(right, 0);
    expect(left, 1);
    expect(
      tester
          .widget<Opacity>(
            find.ancestor(of: _node('W'), matching: find.byType(Opacity)),
          )
          .opacity,
      0.5,
    );
  });

  testWidgets('a selected node takes the wash and the line, the others the '
      'surface and the soft line', (WidgetTester tester) async {
    await _pump(tester, _map());

    final Material on = _material(tester, 'X');
    final Material off = _material(tester, 'W');
    expect(on.color, _shapes.wash(_t.surface));
    expect((on.shape! as CutBorder).side.color, _shapes.line(_t.border));
    expect(off.color, _t.surface);
    expect((off.shape! as CutBorder).side.color, _shapes.lineSoft(_t.border));
  });

  testWidgets('a selected node reads in the colour it is given', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      Column(
        children: <Widget>[
          SizedBox(
            height: kNodeHeight,
            child: _item('on', selected: true, selectedColor: _t.accent),
          ),
          SizedBox(height: kNodeHeight, child: _item('plain', selected: true)),
          SizedBox(height: kNodeHeight, child: _item('off')),
        ],
      ),
    );

    expect(tester.widget<Text>(find.text('on')).style?.color, _t.accent);
    expect(tester.widget<Text>(find.text('plain')).style?.color, _t.text);
    expect(tester.widget<Text>(find.text('off')).style?.color, _t.muted);
  });

  testWidgets('the name keeps its room and the value gives way', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      SizedBox(
        height: kNodeHeight,
        child: _item('Speed', value: 'Normal speed'),
      ),
      width: 140,
    );

    expect(tester.takeException(), isNull);
    expect(
      tester
          .renderObject<RenderParagraph>(
            find.descendant(
              of: find.text('Speed'),
              matching: find.byType(RichText),
            ),
          )
          .didExceedMaxLines,
      isFalse,
    );
    expect(
      tester
          .renderObject<RenderParagraph>(
            find.descendant(
              of: find.text('Normal speed'),
              matching: find.byType(RichText),
            ),
          )
          .didExceedMaxLines,
      isTrue,
    );
  });

  testWidgets('a node opens its choices and a choice is selected, under the '
      'name of the open node', (WidgetTester tester) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(tester, _map());

    expect(
      tester.getSemantics(find.text('A')),
      isSemantics(
        label: 'A',
        isButton: true,
        hasExpandedState: true,
        isExpanded: true,
        hasSelectedState: false,
      ),
    );
    expect(
      tester.getSemantics(find.text('B')),
      isSemantics(
        label: 'B',
        isButton: true,
        hasExpandedState: true,
        isExpanded: false,
      ),
    );
    expect(
      tester.getSemantics(find.text('X')),
      isSemantics(
        label: 'X',
        isButton: true,
        isSelected: true,
        hasExpandedState: false,
      ),
    );
    expect(
      tester.getSemantics(find.text('W')),
      isSemantics(label: 'W', isButton: true, isSelected: false),
    );
    expect(
      find.ancestor(
        of: find.text('W'),
        matching: find.bySemanticsLabel('Picks'),
      ),
      findsOneWidget,
    );
    expect(
      find.ancestor(
        of: find.text('A'),
        matching: find.bySemanticsLabel('Picks'),
      ),
      findsNothing,
    );
    semantics.dispose();
  });

  testWidgets('arrow keys stop at the edge of the map and never leave it', (
    WidgetTester tester,
  ) async {
    final List<LogicalKeyboardKey> outside = <LogicalKeyboardKey>[];
    await _pump(tester, _map(), onOutsideKey: outside.add);
    await _focus(tester, 'Z');

    await _press(tester, LogicalKeyboardKey.arrowDown);
    expect(_focused(), 'Z');

    await _press(tester, LogicalKeyboardKey.arrowRight);
    expect(_focused(), 'Z');

    await _press(tester, LogicalKeyboardKey.arrowLeft);
    expect(_focused(), 'C');

    await _press(tester, LogicalKeyboardKey.arrowLeft);
    expect(_focused(), 'C');

    expect(outside, isEmpty);
  });

  testWidgets('arrow keys across the columns move by position, not back to '
      'where they came from', (WidgetTester tester) async {
    await _pump(tester, _map());
    await _focus(tester, 'Z');

    await _press(tester, LogicalKeyboardKey.arrowLeft);
    expect(_focused(), 'C');
    await _press(tester, LogicalKeyboardKey.arrowUp);
    expect(_focused(), 'B');
    await _press(tester, LogicalKeyboardKey.arrowRight);
    expect(_focused(), 'X');
    await _press(tester, LogicalKeyboardKey.arrowRight);
    expect(_focused(), 'X');

    await _focus(tester, 'Z');
    await _press(tester, LogicalKeyboardKey.arrowLeft);
    await _press(tester, LogicalKeyboardKey.arrowRight);
    expect(_focused(), 'Z');
  });

  testWidgets('tab goes down the nodes, then down the choices', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _map());
    await _focus(tester, 'A');

    final List<String?> order = <String?>[_focused()];
    for (int i = 0; i < 6; i++) {
      await _press(tester, LogicalKeyboardKey.tab);
      order.add(_focused());
    }

    expect(order, <String>['A', 'B', 'C', 'W', 'X', 'Y', 'Z']);
  });

  testWidgets('tab and shift-tab leave the map at either end', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      Column(
        children: <Widget>[
          TextButton(onPressed: () {}, child: const Text('before')),
          _map(),
          TextButton(onPressed: () {}, child: const Text('after')),
        ],
      ),
    );
    String? button() =>
        (FocusManager.instance.primaryFocus?.context
                    ?.findAncestorWidgetOfExactType<TextButton>()
                    ?.child
                as Text?)
            ?.data;

    await _focus(tester, 'Z');
    await _press(tester, LogicalKeyboardKey.tab);
    expect(button(), 'after');

    await _focus(tester, 'A');
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await _press(tester, LogicalKeyboardKey.tab);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    expect(button(), 'before');
  });

  testWidgets('a focused choice keeps focus while the choices are blocked and '
      'freed again', (WidgetTester tester) async {
    await _pump(tester, _map());
    await _focus(tester, 'Y');

    await _pump(tester, _map(rightEnabled: false));
    expect(_focused(), 'Y');

    await _pump(tester, _map());
    expect(_focused(), 'Y');
  });
}
