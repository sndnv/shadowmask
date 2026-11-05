import 'package:flutter/material.dart';

import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/play_on_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/action_segments.dart';
import 'package:shadowmask/components/play_on_button.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/components/version_menu.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/discovery/continue_feed.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/view/failure_reason.dart';
import 'package:shadowmask/view/play_target.dart';
import 'package:shadowmask/view/version_order.dart';

typedef _Resume = ({Version version, int percent});

class PlayButton extends StatefulWidget {
  const PlayButton({
    super.key,
    required this.catalog,
    required this.userId,
    required this.versions,
    this.resumable,
    this.playback,
    this.onProgressCleared,
    this.playOn,
  });

  final CatalogApi catalog;
  final String userId;
  final List<Version> versions;
  final Map<String, int>? resumable;
  final PlaybackApi? playback;
  final VoidCallback? onProgressCleared;
  final PlayOnApi? playOn;

  @override
  State<PlayButton> createState() => PlayButtonState();
}

class PlayButtonState extends State<PlayButton> with PlayOnTargets<PlayButton> {
  static const ButtonStyle _style = kActionButtonStyle;

  final MenuController _menu = MenuController();

  late List<Version> _ordered = orderedVersions(widget.versions);
  late List<Version> _playable = _ordered
      .where((Version v) => v.available)
      .toList();

  late Map<String, int> _inProgress = widget.resumable ?? const <String, int>{};
  Future<Map<String, int>?>? _pending;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _load();
    loadPlayOnTargets(widget.playOn, widget.userId);
  }

  @override
  void didUpdateWidget(PlayButton oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.versions == oldWidget.versions &&
        widget.resumable == oldWidget.resumable) {
      return;
    }
    _ordered = orderedVersions(widget.versions);
    _playable = _ordered.where((Version v) => v.available).toList();
    _load();
  }

  void _load() {
    final Map<String, int>? given = widget.resumable;
    if (given != null) {
      _inProgress = given;
      _pending = null;
      return;
    }
    if (_playable.isEmpty) {
      return;
    }
    final Future<Map<String, int>?> pending = _fetch();
    _pending = pending;
    pending.then((Map<String, int>? found) {
      if (!mounted || !identical(_pending, pending)) {
        return;
      }
      setState(() {
        _inProgress = found ?? const <String, int>{};
        _pending = null;
      });
    });
  }

  Future<Map<String, int>?> _fetch() async {
    try {
      final ContinueFeed feed = await widget.catalog.continueFeed(
        widget.userId,
      );
      return feed.resumeProgress;
    } catch (_) {
      return null;
    }
  }

  void _play(Version v) => Navigator.of(context).pushNamed(watchRoute(v.id));

  Future<Map<String, int>> _latest() async {
    if (_playable.length < 2) {
      return _inProgress;
    }
    final Map<String, int>? found = await (_pending ?? _fetch());
    if (found == null || !mounted) {
      return _inProgress;
    }
    setState(() => _inProgress = found);
    return found;
  }

  Future<Version?> _choice() async =>
      resolvePlayTarget(_playable, (await _latest()).keys.toSet());

  Future<void> press() async {
    if (_busy || _playable.isEmpty) {
      return;
    }
    if (_playable.length == 1) {
      _play(_playable.first);
      return;
    }
    setState(() => _busy = true);
    final Version? target = await _choice();
    if (!mounted) {
      return;
    }
    setState(() => _busy = false);
    if (target != null) {
      _play(target);
    }
  }

  Future<void> _dismiss(Version target) async {
    final PlaybackApi? playback = widget.playback;
    if (playback == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      await playback.clearProgress(widget.userId, target.id);
      if (!mounted) {
        return;
      }
      setState(() {
        _inProgress = <String, int>{..._inProgress}..remove(target.id);
        _busy = false;
      });
      Toasts.of(context).success(Strings.toastResumeDismissed);
      widget.onProgressCleared?.call();
    } catch (e) {
      if (!mounted) {
        return;
      }
      setState(() => _busy = false);
      Toasts.of(context).error(failureText(Strings.errorAction, e));
    }
  }

  @override
  Widget build(BuildContext context) =>
      SelectionContainer.disabled(child: _build(context));

  _Resume? get _resume {
    final Version? target = resolvePlayTarget(
      _playable,
      _inProgress.keys.toSet(),
    );
    if (target == null) {
      return null;
    }
    final int? percent = _inProgress[target.id];
    return percent == null ? null : (version: target, percent: percent);
  }

  Widget _build(BuildContext context) {
    final _Resume? resume = _resume;
    final bool paired = resume != null && widget.playback != null;
    final Widget play = _playControl(resume, paired);
    final bool picks = _playable.length > 1;
    final PlayOnApi? playOn = widget.playOn;
    final bool sends =
        playOn != null && playOnDevices.isNotEmpty && _playable.isNotEmpty;
    if (!paired && !picks && !sends) {
      return play;
    }
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        play,
        if (paired) _dismissButton(resume.version),
        if (picks) _versionButton(),
        if (sends)
          PlayOnButton(playOn: playOn, devices: playOnDevices, choose: _choice),
      ],
    );
  }

  Widget _playControl(_Resume? resume, bool paired) =>
      _button(resume, paired, _playable.isEmpty ? null : press);

  Widget _versionButton() => Padding(
    padding: const EdgeInsetsDirectional.only(start: Space.s2),
    child: VersionMenu(
      controller: _menu,
      ordered: _ordered,
      onPlay: _play,
      child: ToggleButton(
        icon: Icons.playlist_play,
        filledIcon: Icons.playlist_play,
        label: Strings.chooseVersion,
        pressed: false,
        highlight: false,
        busy: _busy,
        compact: true,
        onToggle: () => VersionMenu.toggle(_menu),
      ),
    ),
  );

  Widget _button(_Resume? resume, bool paired, VoidCallback? onPressed) {
    if (resume == null) {
      return FilledButton.icon(
        onPressed: _busy ? null : onPressed,
        style: _style,
        icon: const Icon(Icons.play_arrow, size: 20),
        label: const Text(Strings.play),
      );
    }
    return Tooltip(
      message: Strings.resume(resume.percent),
      child: FilledButton.icon(
        onPressed: _busy ? null : onPressed,
        style: paired ? segmented(_style, kSegmentLeading) : _style,
        icon: const Icon(Icons.play_arrow, size: 20),
        label: const Text(Strings.resumeAction),
      ),
    );
  }

  Widget _dismissButton(Version target) =>
      DismissSegment(onPressed: _busy ? null : () => _dismiss(target));
}
