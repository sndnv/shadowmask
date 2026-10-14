import 'package:flutter/material.dart';

import 'package:shadowmask/api/play_on_api.dart';
import 'package:shadowmask/api/target_caller.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/components/version_label.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/auth/device.dart';
import 'package:shadowmask/model/auth/device_target.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/failure_reason.dart';
import 'package:shadowmask/view/play_target.dart';
import 'package:shadowmask/view/version_order.dart';

mixin PlayOnTargets<T extends StatefulWidget> on State<T> {
  List<Device> playOnDevices = const <Device>[];

  void loadPlayOnTargets(PlayOnApi? playOn, String userId) {
    if (playOn == null) {
      return;
    }
    playOn.targets(userId).then((List<Device> found) {
      if (mounted) {
        setState(() => playOnDevices = found);
      }
    }, onError: (Object _) {});
  }
}

class PlayOnButton extends StatefulWidget {
  const PlayOnButton({
    super.key,
    required this.playOn,
    required this.devices,
    required this.choose,
  }) : plain = false;

  const PlayOnButton.icon({
    super.key,
    required this.playOn,
    required this.devices,
    required this.choose,
  }) : plain = true;

  final PlayOnApi playOn;
  final List<Device> devices;
  final Future<PlayChoice?> Function() choose;
  final bool plain;

  @override
  State<PlayOnButton> createState() => _PlayOnButtonState();
}

class _PlayOnButtonState extends State<PlayOnButton> {
  final MenuController _menu = MenuController();

  List<Version> _ordered = const <Version>[];
  Version? _target;
  bool _busy = false;

  Future<void> _press() async {
    if (_menu.isOpen) {
      _menu.close();
      return;
    }
    setState(() => _busy = true);
    try {
      final PlayChoice? choice = await widget.choose();
      if (!mounted) {
        return;
      }
      if (choice == null || !choice.ordered.any((Version v) => v.available)) {
        Toasts.of(context).error(Strings.nothingToPlay);
        return;
      }
      setState(() {
        _ordered = choice.ordered;
        _target = choice.target;
      });
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) {
          _menu.open();
        }
      });
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorAction, e));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _send(Device device, Version version) async {
    final Toasts toasts = Toasts.of(context);
    final String name = device.name;
    setState(() => _busy = true);
    try {
      switch (await widget.playOn.send(device, version)) {
        case PlayOnOutcome.playing:
          toasts.success(Strings.playOnPlaying(name), emphasis: name);
        case PlayOnOutcome.refused:
          toasts.error(Strings.playOnRefused(name), emphasis: name);
        case PlayOnOutcome.unreachable:
          toasts.error(Strings.playOnUnreachable(name), emphasis: name);
      }
    } catch (e) {
      toasts.error(failureText(Strings.errorAction, e));
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  List<Widget> _items(Tokens t) {
    final Version? target = _target;
    if (target != null) {
      return _deviceItems(t, target);
    }
    return <Widget>[
      for (int i = 0; i < _ordered.length; i++)
        if (_ordered[i].available)
          SubmenuButton(
            menuStyle: appMenuStyle(t),
            menuChildren: _deviceItems(t, _ordered[i]),
            child: Text.rich(
              TextSpan(
                style: monoStyle.copyWith(color: t.text, fontSize: 13),
                children: versionLabelSpans(
                  t,
                  _ordered[i],
                  versionNumberLabel(i),
                ),
              ),
            ),
          ),
    ];
  }

  List<Widget> _deviceItems(Tokens t, Version version) => <Widget>[
    for (final Device d in widget.devices)
      MenuItemButton(
        onPressed: () => _send(d, version),
        leadingIcon: Icon(_kindIcon(d), size: 18, color: t.accent),
        child: Text(d.name),
      ),
  ];

  IconData _kindIcon(Device d) =>
      d.target?.kind == TargetKind.android ? Icons.phone_android : Icons.tv;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Widget anchor = MenuAnchor(
      controller: _menu,
      alignmentOffset: kMenuOffset,
      style: appMenuStyle(t),
      menuChildren: _items(t),
      child: widget.plain
          ? IconButton(
              onPressed: _busy ? null : _press,
              tooltip: Strings.playOn,
              icon: Icon(Icons.devices, color: t.muted),
            )
          : ToggleButton(
              icon: Icons.devices,
              filledIcon: Icons.devices,
              label: Strings.playOn,
              pressed: false,
              highlight: false,
              busy: _busy,
              compact: true,
              onToggle: _press,
            ),
    );
    return widget.plain
        ? anchor
        : Padding(
            padding: const EdgeInsetsDirectional.only(start: Space.s2),
            child: anchor,
          );
  }
}
