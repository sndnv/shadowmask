import 'dart:async';

import 'package:flutter/material.dart';

import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/player_shortcuts.dart';

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
    this.waiting,
    this.waitingDetail,
    this.onKeepWaiting,
    this.onGoBack,
    this.onPlayNow,
    this.ended = false,
    this.onCentrePlay,
    this.onDoubleTapVideo,
    this.onSeekRelative,
    this.onHoldSpeed,
    this.touch = false,
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
  final String? waiting;
  final String? waitingDetail;
  final VoidCallback? onKeepWaiting;
  final VoidCallback? onGoBack;
  final VoidCallback? onPlayNow;
  final bool ended;
  final VoidCallback? onCentrePlay;
  final VoidCallback? onDoubleTapVideo;
  final ValueChanged<int>? onSeekRelative;
  final ValueChanged<double?>? onHoldSpeed;
  final bool touch;
  final bool playing;
  final bool fullscreen;
  final VoidCallback onTapVideo;

  @override
  State<PlayerFrame> createState() => _PlayerFrameState();
}

class _PlayerFrameState extends State<PlayerFrame> {
  bool _visible = true;
  Timer? _hide;
  Offset? _doubleTapAt;
  int _seekDelta = 0;
  bool _seekVisible = false;
  int? _holdStep;
  int _holdShown = 0;
  Timer? _clearSeek;
  Timer? _stepHold;

  @override
  void initState() {
    super.initState();
    _reveal();
  }

  @override
  void dispose() {
    _hide?.cancel();
    _clearSeek?.cancel();
    _stepHold?.cancel();
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

  bool get _canDoubleTap =>
      (widget.touch && widget.onSeekRelative != null) ||
      widget.onDoubleTapVideo != null;

  void _onTap() {
    widget.onTapVideo();
    _reveal();
  }

  void _onDoubleTap(double width) {
    final ValueChanged<int>? seek = widget.onSeekRelative;
    final Offset? at = _doubleTapAt;
    if (widget.touch && seek != null && at != null && width > 0) {
      final int delta = at.dx < width / 2 ? -kSeekStepMs : kSeekStepMs;
      seek(delta);
      _showSeek(delta);
      return;
    }
    widget.onDoubleTapVideo?.call();
    _reveal();
  }

  bool get _canHold => widget.touch && widget.onHoldSpeed != null;

  void _startHold() {
    if (!_canHold) {
      return;
    }
    _applyHold(0);
    _stepHold?.cancel();
    _stepHold = Timer.periodic(kHoldStep, (Timer timer) {
      final int next = (_holdStep ?? 0) + 1;
      if (next >= kHoldSpeeds.length) {
        timer.cancel();
        return;
      }
      _applyHold(next);
    });
  }

  void _endHold() {
    _stepHold?.cancel();
    _stepHold = null;
    if (_holdStep == null) {
      return;
    }
    widget.onHoldSpeed?.call(null);
    setState(() => _holdStep = null);
  }

  void _applyHold(int step) {
    widget.onHoldSpeed!(kHoldSpeeds[step]);
    setState(() {
      _holdStep = step;
      _holdShown = step;
    });
  }

  void _showSeek(int delta) {
    setState(() {
      _seekDelta = delta;
      _seekVisible = true;
    });
    _clearSeek?.cancel();
    _clearSeek = Timer(const Duration(milliseconds: 700), () {
      if (mounted) {
        setState(() => _seekVisible = false);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Duration fade = MediaQuery.disableAnimationsOf(context)
        ? Duration.zero
        : const Duration(milliseconds: 200);
    final EdgeInsets safe = MediaQuery.paddingOf(context);
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
            child: LayoutBuilder(
              builder: (BuildContext context, BoxConstraints c) =>
                  GestureDetector(
                    behavior: HitTestBehavior.opaque,
                    onTap: _onTap,
                    onDoubleTapDown: (TapDownDetails d) =>
                        _doubleTapAt = d.localPosition,
                    onDoubleTap: _canDoubleTap
                        ? () => _onDoubleTap(c.maxWidth)
                        : null,
                    onLongPressStart: _canHold
                        ? (LongPressStartDetails _) => _startHold()
                        : null,
                    onLongPressEnd: _canHold
                        ? (LongPressEndDetails _) => _endHold()
                        : null,
                    onLongPressCancel: _canHold ? _endHold : null,
                  ),
            ),
          ),
        ),
        Positioned.fill(
          child: IgnorePointer(
            child: AnimatedOpacity(
              opacity: _holdStep == null ? 0 : 1,
              duration: fade,
              child: Align(
                alignment: Alignment.topCenter,
                child: Padding(
                  padding: EdgeInsets.only(top: Space.s6 + safe.top),
                  child: _SpeedBadge(rate: kHoldSpeeds[_holdShown]),
                ),
              ),
            ),
          ),
        ),
        Positioned.fill(
          child: IgnorePointer(
            child: AnimatedOpacity(
              opacity: _seekVisible ? 1 : 0,
              duration: fade,
              child: Align(
                alignment: _seekDelta < 0
                    ? Alignment.centerLeft
                    : Alignment.centerRight,
                child: Padding(
                  padding: EdgeInsets.symmetric(
                    horizontal: Space.s6 + safe.horizontal / 2,
                  ),
                  child: _SeekBadge(deltaMs: _seekDelta),
                ),
              ),
            ),
          ),
        ),
        if (widget.diagnostics != null)
          Positioned(
            top: Space.s3,
            right: Space.s3,
            child: widget.diagnostics!,
          ),
        if (widget.waiting != null)
          Positioned.fill(
            child: _Waiting(
              label: widget.waiting!,
              detail: widget.waitingDetail,
              onKeepWaiting: widget.onKeepWaiting,
              onGoBack: widget.onGoBack,
              onPlayNow: widget.onPlayNow,
            ),
          ),
        if (widget.waiting == null &&
            !widget.playing &&
            widget.upNext == null &&
            widget.onCentrePlay != null)
          Positioned.fill(
            child: Center(
              child: _CentreAction(
                ended: widget.ended,
                onPressed: widget.onCentrePlay!,
              ),
            ),
          ),
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
                padding: EdgeInsets.fromLTRB(
                  Space.s3 + safe.left,
                  Space.s3 + safe.top,
                  Space.s3 + safe.right,
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
                        margin: EdgeInsets.only(left: Space.s3 + safe.left),
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

class _CentreAction extends StatelessWidget {
  const _CentreAction({required this.ended, required this.onPressed});

  final bool ended;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: ended ? Strings.playerReplay : Strings.playerPlay,
      child: Material(
        color: const Color(0xB3000000),
        shape: const CircleBorder(),
        child: InkWell(
          customBorder: const CircleBorder(),
          onTap: onPressed,
          child: Padding(
            padding: const EdgeInsets.all(Space.s3),
            child: Icon(
              ended ? Icons.replay : Icons.play_arrow,
              color: Colors.white,
              size: 28,
            ),
          ),
        ),
      ),
    );
  }
}

class _SpeedBadge extends StatelessWidget {
  const _SpeedBadge({required this.rate});

  final double rate;

  @override
  Widget build(BuildContext context) => Semantics(
    liveRegion: true,
    label: Strings.playerHoldSpeed(rate),
    child: Container(
      padding: const EdgeInsets.symmetric(
        horizontal: Space.s3,
        vertical: Space.s2,
      ),
      decoration: const BoxDecoration(
        color: Color(0xB3000000),
        borderRadius: BorderRadius.all(Radius.circular(6)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          const Icon(Icons.fast_forward, color: Colors.white, size: 18),
          const SizedBox(width: Space.s2),
          Text(
            Strings.playerHoldSpeed(rate),
            style: monoStyle.copyWith(color: Colors.white, fontSize: 12),
          ),
        ],
      ),
    ),
  );
}

class _SeekBadge extends StatelessWidget {
  const _SeekBadge({required this.deltaMs});

  final int deltaMs;

  @override
  Widget build(BuildContext context) {
    final bool back = deltaMs < 0;
    return Semantics(
      liveRegion: true,
      label: back ? Strings.shortcutSeekBack : Strings.shortcutSeekForward,
      child: Container(
        padding: const EdgeInsets.symmetric(
          horizontal: Space.s3,
          vertical: Space.s2,
        ),
        decoration: const BoxDecoration(
          color: Color(0xB3000000),
          shape: BoxShape.circle,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(
              back ? Icons.replay_10 : Icons.forward_10,
              color: Colors.white,
              size: 28,
            ),
            Text(
              Strings.playerSeekSeconds(deltaMs.abs() ~/ 1000),
              style: monoStyle.copyWith(color: Colors.white, fontSize: 11),
            ),
          ],
        ),
      ),
    );
  }
}

class _Waiting extends StatelessWidget {
  const _Waiting({
    required this.label,
    this.detail,
    this.onKeepWaiting,
    this.onGoBack,
    this.onPlayNow,
  });

  final String label;
  final String? detail;
  final VoidCallback? onKeepWaiting;
  final VoidCallback? onGoBack;
  final VoidCallback? onPlayNow;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Widget panel = Semantics(
      liveRegion: true,
      label: label,
      child: Center(
        child: Container(
          padding: const EdgeInsets.symmetric(
            horizontal: Space.s4,
            vertical: Space.s3,
          ),
          decoration: BoxDecoration(
            color: const Color(0xB3000000),
            borderRadius: BorderRadius.circular(Space.s2),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (onKeepWaiting == null)
                SizedBox.square(
                  dimension: 28,
                  child: CircularProgressIndicator(
                    strokeWidth: 3,
                    color: t.accent,
                  ),
                )
              else
                Icon(Icons.warning_amber_rounded, color: t.warn, size: 28),
              const SizedBox(height: Space.s2),
              Text(
                label,
                style: const TextStyle(color: Colors.white, fontSize: 13),
              ),
              if (detail != null)
                Text(
                  detail!,
                  style: monoStyle.copyWith(
                    color: Colors.white70,
                    fontSize: 11,
                  ),
                ),
              if (onKeepWaiting != null)
                Padding(
                  padding: const EdgeInsets.only(top: Space.s2),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      if (onGoBack != null)
                        TextButton(
                          onPressed: onGoBack,
                          child: const Text(Strings.playerGoBack),
                        ),
                      TextButton(
                        onPressed: onKeepWaiting,
                        child: const Text(Strings.playerKeepWaiting),
                      ),
                    ],
                  ),
                )
              else if (onPlayNow != null)
                Padding(
                  padding: const EdgeInsets.only(top: Space.s2),
                  child: TextButton(
                    onPressed: onPlayNow,
                    child: const Text(Strings.playerPlayNow),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
    return onKeepWaiting == null && onPlayNow == null
        ? IgnorePointer(child: panel)
        : panel;
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
