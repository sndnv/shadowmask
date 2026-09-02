import 'dart:async';

import 'package:flutter/material.dart';

import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double _barClearance = 84;
const double _minPanelHeight = 140;

class PlayerFrame extends StatefulWidget {
  const PlayerFrame({
    super.key,
    required this.view,
    required this.overlay,
    required this.back,
    required this.playing,
    required this.fullscreen,
    required this.onTapVideo,
    this.diagnostics,
    this.settings,
    this.upNext,
    this.onDismissSettings,
    this.keepVisible = false,
    this.status,
  });

  final Widget view;
  final Widget overlay;
  final Widget back;
  final Widget? diagnostics;
  final Widget? settings;
  final Widget? upNext;
  final VoidCallback? onDismissSettings;
  final bool keepVisible;
  final String? status;
  final bool playing;
  final bool fullscreen;
  final VoidCallback onTapVideo;

  @override
  State<PlayerFrame> createState() => _PlayerFrameState();
}

class _PlayerFrameState extends State<PlayerFrame> {
  bool _visible = true;
  Timer? _hide;

  @override
  void initState() {
    super.initState();
    _reveal();
  }

  @override
  void dispose() {
    _hide?.cancel();
    super.dispose();
  }

  @override
  void didUpdateWidget(PlayerFrame oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.keepVisible && !oldWidget.keepVisible) {
      _reveal();
    }
  }

  void _reveal() {
    if (!_visible) {
      setState(() => _visible = true);
    }
    _hide?.cancel();
    if (widget.keepVisible) {
      return;
    }
    _hide = Timer(const Duration(milliseconds: 2800), () {
      if (mounted && widget.playing) {
        setState(() => _visible = false);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Duration fade = MediaQuery.disableAnimationsOf(context)
        ? Duration.zero
        : const Duration(milliseconds: 200);
    final Widget content = Stack(
      fit: StackFit.expand,
      children: <Widget>[
        Positioned.fill(child: widget.view),
        Positioned.fill(
          child: MouseRegion(
            cursor: widget.fullscreen && !_visible
                ? SystemMouseCursors.none
                : MouseCursor.defer,
            onHover: (_) => _reveal(),
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: () {
                widget.onTapVideo();
                _reveal();
              },
            ),
          ),
        ),
        if (widget.diagnostics != null)
          Positioned(top: Space.s3, left: Space.s3, child: widget.diagnostics!),
        Positioned(
          top: 0,
          left: 0,
          right: 0,
          child: IgnorePointer(
            ignoring: !_visible,
            child: AnimatedOpacity(
              opacity: _visible ? 1 : 0,
              duration: fade,
              child: Container(
                padding: const EdgeInsets.fromLTRB(
                  Space.s3,
                  Space.s3,
                  Space.s3,
                  Space.s5,
                ),
                decoration: const BoxDecoration(
                  gradient: LinearGradient(
                    begin: Alignment.topCenter,
                    end: Alignment.bottomCenter,
                    colors: <Color>[Color(0xB3000000), Color(0x00000000)],
                  ),
                ),
                child: widget.back,
              ),
            ),
          ),
        ),
        Positioned(
          left: 0,
          right: 0,
          bottom: 0,
          child: IgnorePointer(
            ignoring: !_visible,
            child: AnimatedOpacity(
              opacity: _visible ? 1 : 0,
              duration: fade,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  if (widget.status != null)
                    Align(
                      alignment: Alignment.centerLeft,
                      child: Container(
                        margin: const EdgeInsets.only(left: Space.s3),
                        padding: const EdgeInsets.symmetric(
                          horizontal: Space.s2,
                          vertical: 2,
                        ),
                        color: const Color(0x66000000),
                        child: Text(
                          widget.status!,
                          style: monoStyle.copyWith(
                            color: Colors.white,
                            fontSize: 12,
                          ),
                        ),
                      ),
                    ),
                  TapRegion(groupId: kPlayerPanelGroup, child: widget.overlay),
                ],
              ),
            ),
          ),
        ),
        if (widget.upNext != null)
          _AboveBar(
            inset: Space.s4,
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 340),
              child: widget.upNext!,
            ),
          ),
        if (widget.settings != null)
          _AboveBar(
            inset: Space.s3,
            child: TapRegion(
              groupId: kPlayerPanelGroup,
              onTapOutside: (_) => widget.onDismissSettings?.call(),
              child: ConstrainedBox(
                constraints: const BoxConstraints(
                  maxWidth: 340,
                  maxHeight: 360,
                ),
                child: widget.settings!,
              ),
            ),
          ),
      ],
    );

    return Container(
      color: t.artBg,
      child: LayoutBuilder(
        builder: (BuildContext context, BoxConstraints c) => Center(
          child: AspectRatio(
            aspectRatio: _stageRatio(c),
            child: Focus(
              canRequestFocus: false,
              skipTraversal: true,
              onFocusChange: (bool hasFocus) {
                if (hasFocus) {
                  _reveal();
                }
              },
              child: content,
            ),
          ),
        ),
      ),
    );
  }

  double _stageRatio(BoxConstraints c) {
    if (!widget.fullscreen || !c.hasBoundedHeight || c.maxHeight <= 0) {
      return 16 / 9;
    }
    return c.maxWidth / c.maxHeight;
  }
}

class _AboveBar extends StatelessWidget {
  const _AboveBar({required this.inset, required this.child});

  final double inset;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Positioned.fill(
      child: LayoutBuilder(
        builder: (BuildContext context, BoxConstraints c) => Padding(
          padding: EdgeInsets.fromLTRB(
            inset,
            Space.s3,
            inset,
            (c.maxHeight - _minPanelHeight).clamp(Space.s3, _barClearance),
          ),
          child: Align(alignment: Alignment.bottomRight, child: child),
        ),
      ),
    );
  }
}
