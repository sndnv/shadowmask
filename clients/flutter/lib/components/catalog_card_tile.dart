import 'dart:async';

import 'package:flutter/material.dart';

import 'package:shadowmask/components/browser_menu.dart';
import 'package:shadowmask/components/card_menu.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/progress_bar.dart';
import 'package:shadowmask/components/watched_marker.dart';

const double kCardTitleBlockHeight = 20;
const double kCardSubtitleBlockHeight = 17;
const double kCardCaptionBlockHeight = 17;
const double kCardTextBlockHeight =
    kCardTitleBlockHeight +
    kCardSubtitleBlockHeight +
    kCardCaptionBlockHeight +
    Space.s2 * 2;

class CatalogCardTile extends StatefulWidget {
  const CatalogCardTile({
    super.key,
    required this.card,
    required this.imageBase,
    this.width,
    this.onDismiss,
    this.onMenuAction,
    this.dismissBusy = false,
    this.dismissTooltip = Strings.dismiss,
    this.subtitle,
    this.caption,
    this.badge,
    this.badgeTooltip,
    this.aspect,
  });

  final CatalogCard card;
  final String imageBase;
  final double? width;
  final CardAspect? aspect;
  final void Function(CatalogCard card)? onDismiss;
  final void Function(CatalogCard card, CardAction action)? onMenuAction;
  final bool dismissBusy;
  final String dismissTooltip;
  final String? subtitle;
  final String? caption;
  final String? badge;
  final String? badgeTooltip;

  @override
  State<CatalogCardTile> createState() => _CatalogCardTileState();
}

class _CatalogCardTileState extends State<CatalogCardTile> {
  bool _hovered = false;
  bool _focused = false;
  bool _heldBrowserMenu = false;
  Offset? _pressedAt;

  @override
  void dispose() {
    _holdBrowserMenu(false);
    super.dispose();
  }

  void _holdBrowserMenu(bool held) {
    if (held == _heldBrowserMenu) {
      return;
    }
    _heldBrowserMenu = held;
    if (held) {
      BrowserMenu.suppress();
    } else {
      BrowserMenu.restore();
    }
  }

  bool get _dropsResume =>
      widget.onDismiss != null && widget.card.dismissVersionId != null;

  void _openMenu(BuildContext context, Offset at) {
    final CardMenuHost? host = CardMenuHost.maybeOf(context);
    if (host == null) {
      return;
    }
    unawaited(
      showCardMenu(
        context,
        catalog: host.catalog,
        userId: host.userId,
        card: widget.card,
        at: at,
        onDismiss: _dropsResume ? () => widget.onDismiss!(widget.card) : null,
        onChanged: (CardAction action) {
          if (mounted) {
            setState(() {});
          }
          widget.onMenuAction?.call(widget.card, action);
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final CatalogCard card = widget.card;
    final bool menued =
        CardMenuHost.maybeOf(context) != null &&
        hasCardMenu(card.ref.type, dismissible: _dropsResume);
    final String? line2 = widget.subtitle ?? card.subtitle;
    final String? line3 = widget.caption ?? card.caption;
    final bool lit = _hovered || _focused;
    final String tip = <String>[
      card.title,
      ?line2,
      ?line3,
      if (card.progressPercent case final int p) Strings.resume(p),
    ].join('\n');
    return SelectionContainer.disabled(
      child: SizedBox(
        width: widget.width,
        child: Material(
          color: t.surface,
          clipBehavior: Clip.antiAlias,
          animationDuration: Duration.zero,
          shape: RoundedRectangleBorder(
            side: BorderSide(color: lit ? t.accent : t.border),
            borderRadius: const BorderRadius.all(Radii.md),
          ),
          child: InkWell(
            onTap: () => Navigator.of(context).pushNamed(card.route),
            onTapDown: menued
                ? (TapDownDetails d) => _pressedAt = d.globalPosition
                : null,
            onSecondaryTapDown: menued
                ? (TapDownDetails d) => _openMenu(context, d.globalPosition)
                : null,
            onLongPress: menued
                ? () => _openMenu(context, _pressedAt ?? Offset.zero)
                : null,
            onHover: (bool hovered) {
              _holdBrowserMenu(menued && hovered);
              setState(() => _hovered = hovered);
            },
            onFocusChange: (bool focused) => setState(() => _focused = focused),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Stack(
                  children: <Widget>[
                    Tooltip(
                      message: tip,
                      excludeFromSemantics: true,
                      triggerMode: TooltipTriggerMode.manual,
                      child: CardArt(
                        artwork: card.artwork,
                        aspect: widget.aspect ?? card.aspect,
                        imageBase: widget.imageBase,
                        mosaic: card.mosaic,
                        borderRadius: BorderRadius.zero,
                        width: widget.width,
                      ),
                    ),
                    if (widget.badge != null)
                      Positioned(
                        top: Space.s2,
                        left: Space.s2,
                        child: _CountBadge(
                          label: widget.badge!,
                          tooltip: widget.badgeTooltip,
                        ),
                      ),
                    if (card.watched && widget.onDismiss == null)
                      const Positioned(
                        top: Space.s2,
                        right: Space.s2,
                        child: WatchedMarker(),
                      ),
                    if (widget.onDismiss != null)
                      Positioned(
                        top: Space.s2,
                        right: Space.s2,
                        child: _DismissButton(
                          tooltip: widget.dismissTooltip,
                          onPressed: widget.dismissBusy
                              ? null
                              : () => widget.onDismiss!(card),
                        ),
                      ),
                    if (card.progressPercent != null)
                      Positioned(
                        left: 0,
                        right: 0,
                        bottom: 0,
                        child: ProgressBar(percent: card.progressPercent!),
                      ),
                  ],
                ),
                SizedBox(
                  height: kCardTextBlockHeight,
                  child: Tooltip(
                    message: tip,
                    triggerMode: TooltipTriggerMode.manual,
                    child: Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 10,
                        vertical: Space.s2,
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          SizedBox(
                            height: kCardTitleBlockHeight,
                            child: Text(
                              card.title,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: Theme.of(context).textTheme.bodyMedium
                                  ?.copyWith(fontWeight: FontWeight.w700),
                            ),
                          ),
                          SizedBox(
                            height: kCardSubtitleBlockHeight,
                            child: line2 == null
                                ? null
                                : Text(
                                    line2,
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                    style: Theme.of(context).textTheme.bodySmall
                                        ?.copyWith(color: t.muted),
                                  ),
                          ),
                          SizedBox(
                            height: kCardCaptionBlockHeight,
                            child: line3 == null
                                ? null
                                : Text(
                                    line3,
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                    style: Theme.of(context).textTheme.bodySmall
                                        ?.copyWith(color: t.muted),
                                  ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _CountBadge extends StatelessWidget {
  const _CountBadge({required this.label, this.tooltip});

  final String label;
  final String? tooltip;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Widget badge = Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: t.bg.withValues(alpha: 0.75),
        borderRadius: const BorderRadius.all(Radii.sm),
        border: Border.all(color: t.border),
      ),
      child: Text(
        label,
        style: monoStyle.copyWith(
          color: t.text,
          fontSize: 11,
          fontWeight: FontWeight.w700,
        ),
      ),
    );
    final String? message = tooltip;
    return message == null ? badge : Tooltip(message: message, child: badge);
  }
}

class _DismissButton extends StatelessWidget {
  const _DismissButton({required this.tooltip, required this.onPressed});

  final String tooltip;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Tooltip(
      message: tooltip,
      child: Material(
        color: t.surface.withValues(alpha: 0.82),
        shape: const CircleBorder(),
        child: InkWell(
          customBorder: const CircleBorder(),
          onTap: onPressed,
          child: Padding(
            padding: const EdgeInsets.all(3),
            child: Icon(
              Icons.close,
              size: 16,
              color: onPressed == null ? t.muted : t.text,
            ),
          ),
        ),
      ),
    );
  }
}
