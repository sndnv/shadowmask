import 'package:flutter/material.dart';

import 'package:shadowmask/components/menu_option.dart';
import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';

const Color _chrome = Colors.white;
const double _gap = 11;
const double _fullBarWidth = 420;

typedef TransportStep = ({String tooltip, VoidCallback? onPressed});

class OverlayBar extends StatelessWidget {
  const OverlayBar({
    super.key,
    required this.snapshot,
    required this.timeline,
    required this.fullscreen,
    required this.wide,
    required this.muted,
    required this.volume,
    required this.remaining,
    required this.onPlayPause,
    required this.onOpenPanel,
    required this.onToggleFullscreen,
    required this.onToggleWide,
    required this.onToggleMute,
    required this.onVolume,
    required this.onToggleRemaining,
    this.previousEpisode,
    this.nextEpisode,
    this.touch = false,
  });

  final PlayerSnapshot snapshot;
  final Widget timeline;
  final bool fullscreen;
  final bool wide;
  final bool muted;
  final double volume;
  final bool remaining;
  final VoidCallback onPlayPause;
  final ValueChanged<PlayerPanel> onOpenPanel;
  final VoidCallback onToggleFullscreen;
  final VoidCallback? onToggleWide;
  final VoidCallback onToggleMute;
  final ValueChanged<double> onVolume;
  final VoidCallback onToggleRemaining;
  final TransportStep? previousEpisode;
  final TransportStep? nextEpisode;
  final bool touch;

  IconData get _volumeIcon {
    if (muted || volume == 0) {
      return Icons.volume_off;
    }
    return volume < 0.5 ? Icons.volume_down : Icons.volume_up;
  }

  int get _shownMs {
    if (!remaining) {
      return snapshot.positionMs;
    }
    final int left = snapshot.durationMs - snapshot.positionMs;
    return left > 0 ? left : 0;
  }

  @override
  Widget build(BuildContext context) {
    final EdgeInsets safe = MediaQuery.paddingOf(context);
    return Container(
      padding: EdgeInsets.fromLTRB(
        _gap + safe.left,
        Space.s6,
        _gap + safe.right,
        10 + safe.bottom,
      ),
      decoration: const BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.bottomCenter,
          end: Alignment.topCenter,
          colors: <Color>[Color(0xB3000000), Color(0x00000000)],
        ),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Row(
            children: <Widget>[
              Tooltip(
                message: Strings.playerRemainingTime,
                child: InkWell(
                  onTap: onToggleRemaining,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 4,
                      vertical: 2,
                    ),
                    child: Text(
                      '${clock(_shownMs)} / ${clock(snapshot.durationMs)}',
                      style: monoStyle.copyWith(
                        color: const Color(0xFFE8EEF0),
                        fontSize: 12.5,
                      ),
                    ),
                  ),
                ),
              ),
              const SizedBox(width: _gap),
              Expanded(child: timeline),
            ],
          ),
          const SizedBox(height: 6),
          LayoutBuilder(
            builder: (BuildContext context, BoxConstraints constraints) {
              final bool compact =
                  touch || constraints.maxWidth < _fullBarWidth;
              return Row(
                children: <Widget>[
                  if (previousEpisode != null)
                    _IcBtn(
                      icon: Icons.skip_previous,
                      tooltip: previousEpisode!.tooltip,
                      onPressed: previousEpisode!.onPressed,
                      touch: touch,
                    ),
                  _IcBtn(
                    icon: snapshot.playing ? Icons.pause : Icons.play_arrow,
                    tooltip: snapshot.playing
                        ? Strings.playerPause
                        : Strings.playerPlay,
                    onPressed: onPlayPause,
                    touch: touch,
                  ),
                  if (nextEpisode != null)
                    _IcBtn(
                      icon: Icons.skip_next,
                      tooltip: nextEpisode!.tooltip,
                      onPressed: nextEpisode!.onPressed,
                      touch: touch,
                    ),
                  const SizedBox(width: _gap),
                  _IcBtn(
                    icon: _volumeIcon,
                    tooltip: muted ? Strings.playerUnmute : Strings.playerMute,
                    onPressed: onToggleMute,
                    touch: touch,
                  ),
                  if (!compact)
                    SizedBox(
                      width: 76,
                      child: SliderTheme(
                        data: SliderTheme.of(context).copyWith(
                          trackHeight: 3,
                          overlayShape: SliderComponentShape.noOverlay,
                          thumbShape: const RoundSliderThumbShape(
                            enabledThumbRadius: 5,
                          ),
                          activeTrackColor: _chrome,
                          inactiveTrackColor: const Color(0x55FFFFFF),
                          thumbColor: _chrome,
                        ),
                        child: Slider(
                          value: muted ? 0 : volume,
                          onChanged: onVolume,
                        ),
                      ),
                    ),
                  const Spacer(),
                  if (compact)
                    _PanelMenu(onOpenPanel: onOpenPanel, touch: touch)
                  else
                    for (final PlayerPanel panel in PlayerPanel.values)
                      _IcBtn(
                        icon: panel.icon,
                        tooltip: panel.title,
                        onPressed: () => onOpenPanel(panel),
                      ),
                  const SizedBox(width: _gap),
                  if (!touch && !fullscreen && onToggleWide != null)
                    _IcBtn(
                      icon: wide ? Icons.width_normal : Icons.width_wide,
                      tooltip: wide
                          ? Strings.playerNormalScreen
                          : Strings.playerWideScreen,
                      onPressed: onToggleWide,
                    ),
                  if (!touch)
                    _IcBtn(
                      icon: fullscreen
                          ? Icons.fullscreen_exit
                          : Icons.fullscreen,
                      tooltip: Strings.playerFullscreen,
                      onPressed: onToggleFullscreen,
                    ),
                ],
              );
            },
          ),
        ],
      ),
    );
  }
}

class _PanelMenu extends StatefulWidget {
  const _PanelMenu({required this.onOpenPanel, this.touch = false});

  final ValueChanged<PlayerPanel> onOpenPanel;
  final bool touch;

  @override
  State<_PanelMenu> createState() => _PanelMenuState();
}

class _PanelMenuState extends State<_PanelMenu> {
  final MenuController _controller = MenuController();

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return MenuAnchor(
      controller: _controller,
      alignmentOffset: kMenuOffset,
      style: appMenuStyle(t),
      menuChildren: <Widget>[
        for (final PlayerPanel panel in PlayerPanel.values)
          TapRegion(
            groupId: kPlayerPanelGroup,
            child: MenuOption(
              label: panel.title,
              leading: Icon(panel.icon, size: 16, color: t.muted),
              onTap: () {
                _controller.close();
                widget.onOpenPanel(panel);
              },
            ),
          ),
      ],
      builder: (BuildContext context, MenuController controller, Widget? _) =>
          _IcBtn(
            icon: Icons.tune,
            tooltip: Strings.playerOptions,
            onPressed: () =>
                controller.isOpen ? controller.close() : controller.open(),
            touch: widget.touch,
          ),
    );
  }
}

class _IcBtn extends StatelessWidget {
  const _IcBtn({
    required this.icon,
    required this.tooltip,
    required this.onPressed,
    this.touch = false,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;
  final bool touch;

  @override
  Widget build(BuildContext context) {
    return IconButton(
      onPressed: onPressed,
      color: _chrome,
      iconSize: touch ? 24 : 20,
      padding: EdgeInsets.all(touch ? 10 : 6),
      visualDensity: touch ? VisualDensity.standard : VisualDensity.compact,
      constraints: BoxConstraints(
        minWidth: touch ? 44 : 32,
        minHeight: touch ? 44 : 32,
      ),
      tooltip: tooltip,
      icon: Icon(icon),
    );
  }
}
