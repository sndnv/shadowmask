import 'package:flutter/material.dart';

import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kSheetShare = 0.4;
const double kSheetMinWidth = 320;
const double kSheetMaxWidth = 420;
const double kSheetSectionHeight = 56;

double playerSheetWidth(double frameWidth) =>
    (frameWidth * kSheetShare).clamp(kSheetMinWidth, kSheetMaxWidth);

typedef SheetSection =
    Widget Function(PlayerPanel panel, ChoiceOpener openChoice);

class PlayerSheet extends StatefulWidget {
  const PlayerSheet({
    super.key,
    required this.section,
    required this.summary,
    required this.onClose,
  });

  final SheetSection section;
  final String? Function(PlayerPanel panel) summary;
  final VoidCallback onClose;

  @override
  State<PlayerSheet> createState() => _PlayerSheetState();
}

class _PlayerSheetState extends State<PlayerSheet> {
  PlayerPanel? _panel;
  PlayerChoice? _choice;

  void _back() => setState(() {
    if (_choice != null) {
      _choice = null;
    } else {
      _panel = null;
    }
  });

  void _open(PlayerChoice choice) => setState(() => _choice = choice);

  void _pick(PlayerChoice choice, int index) {
    choice.onPick(index);
    setState(() => _choice = null);
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final EdgeInsets safe = MediaQuery.paddingOf(context);
    final PlayerPanel? panel = _panel;
    final PlayerChoice? choice = _choice;
    return Material(
      color: t.surface,
      child: Padding(
        padding: EdgeInsets.only(
          top: safe.top,
          bottom: safe.bottom,
          right: safe.right,
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            _header(
              t,
              choice?.title ?? panel?.title ?? Strings.playerOptions,
              panel != null,
            ),
            Expanded(
              child: panel == null
                  ? _sections(t)
                  : IndexedStack(
                      index: choice == null ? 0 : 1,
                      sizing: StackFit.expand,
                      children: <Widget>[
                        widget.section(panel, _open),
                        if (choice == null)
                          const SizedBox.shrink()
                        else
                          _choices(t, choice),
                      ],
                    ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _header(Tokens t, String title, bool canGoBack) => DecoratedBox(
    decoration: BoxDecoration(
      border: Border(bottom: BorderSide(color: t.border)),
    ),
    child: SizedBox(
      height: kSheetSectionHeight,
      child: Row(
        children: <Widget>[
          if (canGoBack)
            IconButton(
              onPressed: _back,
              tooltip: Strings.back,
              icon: Icon(Icons.arrow_back, color: t.text),
            )
          else
            const SizedBox(width: Space.s4),
          Expanded(
            child: Text(
              title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                color: t.text,
                fontSize: 17,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
          IconButton(
            onPressed: widget.onClose,
            tooltip: Strings.close,
            icon: Icon(Icons.close, color: t.muted),
          ),
        ],
      ),
    ),
  );

  Widget _sections(Tokens t) => ListView(
    padding: const EdgeInsets.symmetric(vertical: Space.s2),
    children: <Widget>[
      for (final PlayerPanel panel in PlayerPanel.values)
        InkWell(
          onTap: () => setState(() => _panel = panel),
          child: SizedBox(
            height: kSheetSectionHeight,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: Space.s4),
              child: Row(
                children: <Widget>[
                  Icon(panel.icon, size: 22, color: t.muted),
                  const SizedBox(width: Space.s3),
                  Text(
                    panel.title,
                    style: TextStyle(color: t.text, fontSize: 16),
                  ),
                  const SizedBox(width: Space.s3),
                  Expanded(
                    child: Text(
                      widget.summary(panel) ?? '',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      textAlign: TextAlign.right,
                      style: TextStyle(color: t.muted, fontSize: 15),
                    ),
                  ),
                  Icon(Icons.chevron_right, color: t.muted),
                ],
              ),
            ),
          ),
        ),
    ],
  );

  Widget _choices(Tokens t, PlayerChoice choice) => ListView(
    padding: const EdgeInsets.symmetric(vertical: Space.s2),
    children: <Widget>[
      for (int i = 0; i < choice.labels.length; i++)
        InkWell(
          onTap: () => _pick(choice, i),
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: kSheetRowHeight),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: Space.s4),
              child: Row(
                children: <Widget>[
                  SizedBox(
                    width: 24,
                    child: i == choice.selected
                        ? Icon(Icons.check, color: t.accent)
                        : null,
                  ),
                  const SizedBox(width: Space.s3),
                  Expanded(
                    child: Text(
                      choice.labels[i],
                      style: TextStyle(
                        color: i == choice.selected ? t.accent : t.text,
                        fontSize: 16,
                        fontWeight: i == choice.selected
                            ? FontWeight.w600
                            : FontWeight.w400,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
    ],
  );
}
