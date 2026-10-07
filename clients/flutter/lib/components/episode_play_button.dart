import 'package:flutter/material.dart';

import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/action_segments.dart';
import 'package:shadowmask/components/muted_note.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/version_menu.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/discovery/continue_feed.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/view/failure_reason.dart';
import 'package:shadowmask/view/next_episode.dart';
import 'package:shadowmask/view/play_target.dart';
import 'package:shadowmask/view/version_order.dart';

class NextEpisodeNote extends StatelessWidget {
  const NextEpisodeNote({
    super.key,
    required this.next,
    required this.withSeason,
  });

  final Future<NextEpisode?> next;
  final bool withSeason;

  @override
  Widget build(BuildContext context) => FutureBuilder<NextEpisode?>(
    future: next,
    builder: (BuildContext context, AsyncSnapshot<NextEpisode?> snapshot) {
      final NextEpisode? found = snapshot.data;
      return Semantics(
        label: found == null
            ? null
            : nextEpisodeSpoken(found, withSeason: withSeason),
        child: ExcludeSemantics(
          child: found == null
              ? const MutedNote('', maxLines: 1)
              : MutedNote.rich(
                  TextSpan(
                    children: <InlineSpan>[
                      const TextSpan(text: Strings.upNextLead),
                      TextSpan(
                        text: nextEpisodeName(found, withSeason: withSeason),
                        style: const TextStyle(fontWeight: FontWeight.w700),
                      ),
                    ],
                  ),
                  maxLines: 1,
                ),
        ),
      );
    },
  );
}

class EpisodePlayButton extends StatefulWidget {
  const EpisodePlayButton({
    super.key,
    required this.catalog,
    required this.userId,
    required this.seriesId,
    required this.next,
    required this.withSeason,
  });

  final CatalogApi catalog;
  final String userId;
  final String seriesId;
  final Future<NextEpisode?> next;
  final bool withSeason;

  @override
  State<EpisodePlayButton> createState() => EpisodePlayButtonState();
}

class EpisodePlayButtonState extends State<EpisodePlayButton> {
  final MenuController _menu = MenuController();

  NextEpisode? _next;
  List<Version> _ordered = const <Version>[];
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _listen();
  }

  @override
  void didUpdateWidget(EpisodePlayButton oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(widget.next, oldWidget.next)) {
      _next = null;
      _listen();
    }
  }

  void _listen() {
    final Future<NextEpisode?> pending = widget.next;
    pending.then((NextEpisode? found) {
      if (!mounted || !identical(widget.next, pending)) {
        return;
      }
      setState(() => _next = found);
    });
  }

  Future<Map<String, int>> _resumable() async {
    try {
      final ContinueFeed feed = await widget.catalog.continueFeed(
        widget.userId,
      );
      return feed.resumeProgress;
    } catch (_) {
      return const <String, int>{};
    }
  }

  void _play(Version v) => Navigator.of(context).pushNamed(watchRoute(v.id));

  Future<void> press() async {
    if (_busy) {
      return;
    }
    setState(() => _busy = true);
    try {
      final NextEpisode? next = await widget.next;
      if (!mounted) {
        return;
      }
      if (next == null) {
        Toasts.of(context).error(Strings.nothingToPlay);
        return;
      }
      final Future<Map<String, int>> resumable = _resumable();
      final List<Version> versions = (await widget.catalog.episodeVersions(
        next.episode.id,
        series: widget.seriesId,
        season: next.episode.seasonId,
      )).items;
      final Map<String, int> inProgress = await resumable;
      if (!mounted) {
        return;
      }
      _choose(orderedVersions(versions), inProgress);
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

  void _choose(List<Version> ordered, Map<String, int> inProgress) {
    final List<Version> available = ordered
        .where((Version v) => v.available)
        .toList();
    if (available.isEmpty) {
      Toasts.of(context).error(Strings.nothingToPlay);
      return;
    }
    final Version? target = resolvePlayTarget(
      available,
      inProgress.keys.toSet(),
    );
    if (target != null) {
      _play(target);
      return;
    }
    setState(() => _ordered = ordered);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _menu.open();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final NextEpisode? next = _next;
    final Widget button = FilledButton.icon(
      onPressed: _busy ? null : press,
      style: kActionButtonStyle,
      icon: const Icon(Icons.play_arrow, size: 20),
      label: Text(nextEpisodeAction(next)),
    );
    return SelectionContainer.disabled(
      child: VersionMenu(
        controller: _menu,
        ordered: _ordered,
        onPlay: _play,
        child: next == null
            ? button
            : Tooltip(
                message: nextEpisodeCaption(
                  next,
                  withSeason: widget.withSeason,
                ),
                excludeFromSemantics: true,
                child: button,
              ),
      ),
    );
  }
}
