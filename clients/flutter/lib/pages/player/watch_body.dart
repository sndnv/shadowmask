import 'dart:async';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/player/diagnostics_overlay.dart';
import 'package:shadowmask/components/player/overlay_bar.dart';
import 'package:shadowmask/components/player/player_frame.dart';
import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/components/player/shortcuts_dialog.dart';
import 'package:shadowmask/components/player/timeline.dart';
import 'package:shadowmask/components/player/up_next_card.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/player/trickplay_loader.dart';
import 'package:shadowmask/components/player/trickplay_thumb.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/movie_detail.dart';
import 'package:shadowmask/model/catalog/season.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/server/server_info.dart';
import 'package:shadowmask/model/session/playback_session.dart';
import 'package:shadowmask/model/session/resume_position.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/immersive_scope.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/player/web_history.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/util/scoped_value.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shadowmask/view/episode_neighbours.dart';
import 'package:shadowmask/view/play_target.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/player_shortcuts.dart';

class WatchBody extends StatefulWidget {
  const WatchBody({
    super.key,
    required this.api,
    required this.user,
    required this.versionId,
    required this.initialControls,
    required this.controllerFactory,
    required this.wide,
    required this.onToggleWide,
  });

  final ApiClient api;
  final SelfUser user;
  final String versionId;
  final PlaybackControls initialControls;
  final PlayerControllerFactory controllerFactory;
  final bool wide;
  final VoidCallback? onToggleWide;

  @override
  State<WatchBody> createState() => _WatchBodyState();
}

class _WatchBodyState extends State<WatchBody> with RouteAware {
  late final PlaybackApi _pb = PlaybackApi(widget.api);
  late final PlayerController _controller = widget.controllerFactory(
    _pb.baseUrl,
  );
  static const PlayerPrefsStore _prefs = PlayerPrefsStore();
  late Future<bool> _boot = _start();
  bool _unloadBound = false;

  PlaybackControls _controls = const PlaybackControls();
  double _speed = 1;
  bool _diag = false;
  bool _remaining = false;
  PlayerPanel? _panel;
  PlaybackSession? _session;
  VersionDetail? _version;
  TrickplayLoader? _trickplay;
  Timer? _heartbeat;
  String? _status;
  String? _mode;
  String? _subtitleDelivery;
  String? _title;
  String? _detailRoute;
  List<Crumb> _crumbs = const <Crumb>[];
  String? _lastState;
  ScopedValue<bool>? _immersive;
  int _autoplaySeconds = kDefaultPlayerPrefs.autoplaySeconds;
  String? _seriesId;
  EpisodeNeighbours _neighbours = const EpisodeNeighbours();
  EpisodeLink? _upNext;
  int _upNextLeft = 0;
  Timer? _countdown;
  bool _autoplayHandled = false;
  final FocusNode _keys = FocusNode(debugLabel: 'player-shortcuts');

  @override
  void initState() {
    super.initState();
    _controls = widget.initialControls;
    _controller.snapshot.addListener(_onSnapshot);
    _controller.fullscreen.addListener(_onFullscreen);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _immersive = ImmersiveScope.of(context);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _onFullscreen();
      }
    });
    final ModalRoute<dynamic>? route = ModalRoute.of(context);
    if (route is PageRoute<dynamic>) {
      appRouteObserver.subscribe(this, route);
    }
  }

  void _onFullscreen() =>
      _immersive?.publish(_controller.fullscreen.value, owner: this);

  @override
  void didPushNext() => _release();

  @override
  void didPopNext() {
    if (_session != null || !mounted) {
      return;
    }
    setState(() {
      _boot = _start();
    });
  }

  void _release() {
    _heartbeat?.cancel();
    _heartbeat = null;
    _cancelCountdown();
    _controller.pause();
    _endSession();
  }

  void _endSession() {
    final String? sid = _session?.sessionId;
    if (sid == null) {
      return;
    }
    _flushProgress();
    _pb.endSession(sid);
    _session = null;
  }

  @override
  void dispose() {
    appRouteObserver.unsubscribe(this);
    _controller.fullscreen.removeListener(_onFullscreen);
    _immersive?.releaseAfterFrame(true, false, owner: this);
    _controller.snapshot.removeListener(_onSnapshot);
    _countdown?.cancel();
    _keys.dispose();
    _heartbeat?.cancel();
    _endSession();
    _trickplay?.dispose();
    _controller.dispose();
    super.dispose();
  }

  Future<bool> _start() async {
    final ServerInfo info = await _pb.serverInfo().catchError(
      (_) => const ServerInfo(),
    );
    final ResumePosition resume = await _pb
        .resume(widget.user.id, widget.versionId)
        .catchError((_) => const ResumePosition());
    final VersionDetail version = await CatalogApi(
      widget.api,
    ).version(widget.versionId);
    final PlaybackSession session = await _pb.startSession(
      versionId: widget.versionId,
      startPositionMs: resume.positionMs,
      profileVersion: info.profileVersion,
    );
    _session = session;
    _version = version;
    _mode = session.mode.name;
    _subtitleDelivery = session.selected?.subtitleDelivery;
    _trickplay = TrickplayLoader(
      api: _pb,
      versionId: widget.versionId,
      refs: session.trickplay.isNotEmpty
          ? session.trickplay
          : version.trickplay,
    );
    await _controller.attach(
      session.manifestUrl,
      mode: session.mode,
      positionMs: resume.positionMs,
    );
    final PlayerPrefs prefs = await _prefs.load().catchError(
      (_) => kDefaultPlayerPrefs,
    );
    _remaining = prefs.remaining;
    _autoplaySeconds = prefs.autoplaySeconds;
    _controller.setVolume(prefs.volume);
    if (prefs.muted) {
      _controller.setMuted(true);
    }
    _startHeartbeat(session.heartbeatIntervalS);
    if (!_unloadBound) {
      _unloadBound = true;
      addUnloadListener(_endSessionBeacon);
    }
    if (!_controls.isDefault) {
      await _applyControls(_controls, seekMs: resume.positionMs);
    }
    await _loadTitle(version.title);
    return true;
  }

  Future<void> _loadTitle(TitleRef ref) async {
    final CatalogApi catalog = CatalogApi(widget.api);
    if (ref.type == TitleKind.movie) {
      try {
        final MovieDetail movie = await catalog.movie(ref.id);
        _title = movie.year != null
            ? Strings.titleWithYear(movie.title, movie.year!)
            : movie.title;
        _detailRoute = movieRoute(movie.id);
        _crumbs = <Crumb>[
          Crumb(Strings.navigationMovies, route: moviesRoute()),
          Crumb(movie.title, route: _detailRoute),
          const Crumb(Strings.watchHeading),
        ];
      } catch (_) {}
      return;
    }
    try {
      final Episode episode = await catalog.episode(ref.id);
      Season? season;
      try {
        season = await catalog.season(episode.seasonId);
      } catch (_) {}
      final String? seriesId = season?.seriesId ?? episode.seriesId;
      String? seriesTitle = episode.seriesTitle;
      if (seriesId != null) {
        try {
          seriesTitle = (await catalog.seriesDetail(seriesId)).title;
        } catch (_) {}
      }
      final String label = Strings.episodeTitle(episode.number, episode.title);
      _title = seriesTitle != null ? '$seriesTitle - $label' : label;
      _seriesId = seriesId;
      _neighbours = await loadNeighbours(catalog, episode);
      _detailRoute = episodeRoute(episode.id, series: seriesId);
      final int? seasonNumber = season?.number ?? episode.seasonNumber;
      _crumbs = <Crumb>[
        Crumb(Strings.navigationSeries, route: seriesListRoute()),
        if (seriesId != null)
          Crumb(seriesTitle ?? seriesId, route: seriesRoute(seriesId)),
        Crumb(
          season?.title ??
              (seasonNumber != null
                  ? Strings.seasonLabel(seasonNumber)
                  : Strings.seasonsHeading),
          route: seasonRoute(episode.seasonId, series: seriesId),
        ),
        Crumb(label, route: _detailRoute),
        const Crumb(Strings.watchHeading),
      ];
    } catch (_) {}
  }

  void _startHeartbeat(int intervalS) {
    _heartbeat = Timer.periodic(Duration(seconds: max(1, intervalS)), (_) {
      _sendProgress(_controller.snapshot.value.playing ? 'playing' : 'paused');
    });
  }

  void _onSnapshot() {
    if (!mounted) {
      return;
    }
    final PlayerSnapshot s = _controller.snapshot.value;
    final String state = s.playing ? 'playing' : 'paused';
    if (state != _lastState) {
      _lastState = state;
      _sendProgress(state);
    }
    if (s.playing) {
      _autoplayHandled = false;
      _cancelCountdown();
      return;
    }
    if (s.ended) {
      _armAutoplay();
      return;
    }
    _autoplayHandled = false;
  }

  void _armAutoplay() {
    final EpisodeLink? next = _neighbours.next;
    if (_autoplayHandled || _autoplaySeconds <= 0 || next == null) {
      return;
    }
    _autoplayHandled = true;
    setState(() {
      _upNext = next;
      _upNextLeft = _autoplaySeconds;
    });
    _countdown = Timer.periodic(const Duration(seconds: 1), (Timer timer) {
      if (!mounted) {
        return;
      }
      if (_upNextLeft <= 1) {
        _cancelCountdown();
        _goToEpisode(next);
        return;
      }
      setState(() => _upNextLeft -= 1);
    });
  }

  void _cancelCountdown() {
    if (_countdown == null && _upNext == null) {
      return;
    }
    _countdown?.cancel();
    _countdown = null;
    if (mounted) {
      setState(() => _upNext = null);
    }
  }

  Future<void> _goToEpisode(EpisodeLink link) async {
    _cancelCountdown();
    final String route = await _episodeRoute(link);
    if (mounted) {
      await Navigator.of(context).pushReplacementNamed(route);
    }
  }

  Future<String> _episodeRoute(EpisodeLink link) async {
    try {
      final Paged<Version> versions = await CatalogApi(
        widget.api,
      ).episodeVersions(link.id, series: _seriesId, season: link.seasonId);
      final Version? target = resolvePlayTarget(
        versions.items.where((Version v) => v.available).toList(),
        const <String>{},
      );
      if (target != null) {
        return watchRoute(target.id);
      }
    } catch (_) {}
    return episodeRoute(link.id, series: _seriesId);
  }

  TransportStep? _step(
    EpisodeLink? link,
    String none,
    String Function(String) named,
  ) {
    if (_neighbours.previous == null && _neighbours.next == null) {
      return null;
    }
    return (
      tooltip: link == null ? none : named(link.label),
      onPressed: link == null ? null : () => _goToEpisode(link),
    );
  }

  void _sendProgress(String state) {
    final String? sid = _session?.sessionId;
    if (sid != null) {
      _pb.progress(sid, _controller.snapshot.value.positionMs, state);
    }
  }

  void _flushProgress() {
    _sendProgress(_controller.snapshot.value.playing ? 'playing' : 'paused');
  }

  void _endSessionBeacon() => _endSession();

  void _writeUrl() {
    replaceUrl(
      withQuery('/watch', <String, String?>{
        'version': widget.versionId,
        ..._controls.toQuery(),
      }),
    );
  }

  Future<void> _applyControls(PlaybackControls next, {int? seekMs}) async {
    setState(() => _controls = next);
    _writeUrl();
    final String? sid = _session?.sessionId;
    if (sid == null) {
      return;
    }
    final int keepMs = seekMs ?? _controller.snapshot.value.positionMs;
    try {
      final neg = await _pb.update(sid, next);
      await _controller.attach(
        neg.manifestUrl,
        mode: neg.mode,
        positionMs: keepMs,
      );
      if (mounted) {
        setState(() {
          _mode = neg.mode.name;
          _subtitleDelivery = neg.selected?.subtitleDelivery;
          _status = null;
        });
      }
    } catch (_) {
      if (mounted) {
        setState(() => _status = Strings.renegotiationFailed);
      }
    }
  }

  void _changeSpeed(double rate) {
    setState(() => _speed = rate);
    _controller.setRate(rate);
  }

  void _changeDiagnostics(bool on) => setState(() => _diag = on);

  void _openPanel(PlayerPanel panel) {
    if (_version == null) {
      return;
    }
    setState(() => _panel = _panel == panel ? null : panel);
  }

  void _closePanel() {
    setState(() => _panel = null);
    _reclaimKeys();
  }

  void _changeRemaining(bool on) {
    setState(() => _remaining = on);
    _prefs.saveRemaining(on);
  }

  void _changeAutoplay(int seconds) {
    setState(() => _autoplaySeconds = seconds);
    _prefs.saveAutoplaySeconds(seconds);
    if (seconds <= 0) {
      _cancelCountdown();
    }
  }

  bool get _typing {
    final BuildContext? focused = FocusManager.instance.primaryFocus?.context;
    if (focused == null) {
      return false;
    }
    return focused.widget is EditableText ||
        focused.findAncestorWidgetOfExactType<EditableText>() != null;
  }

  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    if (_typing) {
      return KeyEventResult.ignored;
    }
    final int? fraction = seekFractionFor(event.logicalKey);
    if (fraction != null) {
      _seekBy(null, toFraction: fraction / 10);
      return KeyEventResult.handled;
    }
    final PlayerShortcut? action = shortcutFor(event.logicalKey);
    if (action == null) {
      return KeyEventResult.ignored;
    }
    _run(action);
    return KeyEventResult.handled;
  }

  void _run(PlayerShortcut action) {
    switch (action) {
      case PlayerShortcut.playPause:
        _controller.togglePlay();
      case PlayerShortcut.seekBack:
        _seekBy(-kSeekStepMs);
      case PlayerShortcut.seekForward:
        _seekBy(kSeekStepMs);
      case PlayerShortcut.volumeUp:
        _nudgeVolume(kVolumeStep);
      case PlayerShortcut.volumeDown:
        _nudgeVolume(-kVolumeStep);
      case PlayerShortcut.mute:
        _toggleMute();
      case PlayerShortcut.fullscreen:
        _controller.toggleFullscreen();
      case PlayerShortcut.wide:
        widget.onToggleWide?.call();
      case PlayerShortcut.previousEpisode:
        _stepEpisode(_neighbours.previous);
      case PlayerShortcut.nextEpisode:
        _stepEpisode(_neighbours.next);
      case PlayerShortcut.remaining:
        _changeRemaining(!_remaining);
      case PlayerShortcut.legend:
        _showShortcuts();
    }
  }

  Future<void> _showShortcuts() async {
    _closePanel();
    await showShortcutsDialog(context);
    _reclaimKeys();
  }

  void _stepEpisode(EpisodeLink? link) {
    if (link != null) {
      _goToEpisode(link);
    }
  }

  void _seekBy(int? deltaMs, {double? toFraction}) {
    final PlayerSnapshot s = _controller.snapshot.value;
    if (s.durationMs <= 0) {
      return;
    }
    final int target = toFraction != null
        ? (s.durationMs * toFraction).round()
        : s.positionMs + deltaMs!;
    _controller.seekTo(target.clamp(0, s.durationMs));
  }

  void _nudgeVolume(double by) {
    _changeVolume((_controller.volume.value + by).clamp(0.0, 1.0));
  }

  void _changeVolume(double level) {
    _controller.setVolume(level);
    _prefs.saveVolume(level);
    _prefs.saveMuted(_controller.muted.value);
  }

  void _toggleMute() {
    final bool next = !_controller.muted.value;
    _controller.setMuted(next);
    _prefs.saveMuted(next);
    if (!next) {
      _prefs.saveVolume(_controller.volume.value);
    }
  }

  void _reclaimKeys() {
    if (!mounted || _keys.hasFocus) {
      return;
    }
    if (ModalRoute.of(context)?.isCurrent ?? true) {
      _keys.requestFocus();
    }
  }

  void _onTapVideo() {
    _reclaimKeys();
    if (_panel != null) {
      setState(() => _panel = null);
      return;
    }
    if (_controller.mutedByPolicy.value) {
      _controller.unmute();
      return;
    }
    _controller.togglePlay();
  }

  String? _sourceLine() {
    final VersionDetail? version = _version;
    if (version == null || version.video.isEmpty) {
      return null;
    }
    final VideoTrack v = version.video.first;
    return 'source: ${v.codec} ${v.width}x${v.height} '
        '@ ${v.frameRate.toStringAsFixed(0)}fps';
  }

  Widget _titleRow() => Padding(
    padding: const EdgeInsets.symmetric(horizontal: Space.s2),
    child: Text(
      _title ?? '',
      overflow: TextOverflow.ellipsis,
      style: const TextStyle(color: Colors.white, fontWeight: FontWeight.w600),
    ),
  );

  @override
  Widget build(BuildContext context) {
    return buildBlock<bool>(
      future: _boot,
      errorText: Strings.couldNotStartPlayback,
      builder: (BuildContext context, bool _) {
        return ValueListenableBuilder<bool>(
          valueListenable: _controller.fullscreen,
          builder: (BuildContext context, bool fullscreen, _) {
            return ValueListenableBuilder<PlayerSnapshot>(
              valueListenable: _controller.snapshot,
              builder: (BuildContext context, PlayerSnapshot snap, _) {
                final double buffered = snap.durationMs > 0
                    ? ((snap.positionMs + snap.bufferedAheadMs) /
                              snap.durationMs)
                          .clamp(0.0, 1.0)
                    : 0.0;
                final TrickplayLoader? tp = _trickplay;
                final Timeline timeline = Timeline(
                  fraction: snap.fraction,
                  bufferedFraction: buffered,
                  durationMs: snap.durationMs,
                  onSeek: (double f) =>
                      _controller.seekTo((f * snap.durationMs).round()),
                  thumbBuilder: (tp != null && tp.available)
                      ? (int ms) => TrickplayThumb(
                          tile: tp.tileFor(ms, () {
                            if (mounted) {
                              setState(() {});
                            }
                          }),
                          positionMs: ms,
                        )
                      : null,
                );
                final VersionDetail? version = _version;
                final Widget frame = Focus(
                  autofocus: true,
                  focusNode: _keys,
                  onKeyEvent: _onKey,
                  child: PlayerFrame(
                    view: _controller.view,
                    playing: snap.playing,
                    fullscreen: fullscreen,
                    onTapVideo: _onTapVideo,
                    keepVisible: _panel != null,
                    status: snap.error ?? _status,
                    diagnostics: _diag
                        ? DiagnosticsOverlay(
                            controller: _controller,
                            sourceLine: _sourceLine(),
                          )
                        : null,
                    settings: _panel != null && version != null
                        ? ValueListenableBuilder<bool>(
                            valueListenable: _controller.pictureInPicture,
                            builder:
                                (BuildContext context, bool pip, Widget? _) =>
                                    PlayerSettingsPanel(
                                      panel: _panel!,
                                      controls: _controls,
                                      version: version,
                                      speed: _speed,
                                      diagnostics: _diag,
                                      mode: _mode,
                                      subtitleDelivery: _subtitleDelivery,
                                      pictureInPicture:
                                          _controller.supportsPictureInPicture
                                          ? pip
                                          : null,
                                      onPictureInPicture:
                                          _controller.togglePictureInPicture,
                                      onControls: (PlaybackControls next) =>
                                          _applyControls(next),
                                      onSpeed: _changeSpeed,
                                      onDiagnostics: _changeDiagnostics,
                                      autoplaySeconds: _neighbours.next != null
                                          ? _autoplaySeconds
                                          : null,
                                      onAutoplaySeconds: _changeAutoplay,
                                      onShortcuts: _showShortcuts,
                                      onClose: _closePanel,
                                      dense: compactViewport(context),
                                    ),
                          )
                        : null,
                    onDismissSettings: _closePanel,
                    upNext: _upNext == null
                        ? null
                        : UpNextCard(
                            label: _upNext!.label,
                            remainingSeconds: _upNextLeft,
                            onPlay: () => _goToEpisode(_upNext!),
                            onCancel: _cancelCountdown,
                          ),
                    back: _titleRow(),
                    overlay: ValueListenableBuilder<bool>(
                      valueListenable: _controller.muted,
                      builder: (BuildContext context, bool muted, Widget? _) =>
                          ValueListenableBuilder<double>(
                            valueListenable: _controller.volume,
                            builder:
                                (
                                  BuildContext context,
                                  double level,
                                  Widget? _,
                                ) => OverlayBar(
                                  snapshot: snap,
                                  timeline: timeline,
                                  fullscreen: fullscreen,
                                  wide: widget.wide,
                                  muted: muted,
                                  volume: level,
                                  remaining: _remaining,
                                  onPlayPause: _controller.togglePlay,
                                  onOpenPanel: _openPanel,
                                  onToggleFullscreen:
                                      _controller.toggleFullscreen,
                                  onToggleWide: widget.onToggleWide,
                                  onToggleMute: _toggleMute,
                                  onVolume: _changeVolume,
                                  onToggleRemaining: () =>
                                      _changeRemaining(!_remaining),
                                  previousEpisode: _step(
                                    _neighbours.previous,
                                    Strings.noEarlierEpisode,
                                    Strings.previousNamed,
                                  ),
                                  nextEpisode: _step(
                                    _neighbours.next,
                                    Strings.noLaterEpisode,
                                    Strings.nextNamed,
                                  ),
                                ),
                          ),
                    ),
                  ),
                );
                return LayoutBuilder(
                  builder: (BuildContext context, BoxConstraints c) => Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Offstage(
                        offstage: fullscreen,
                        child: Breadcrumbs(_crumbs),
                      ),
                      Flexible(
                        flex: c.hasBoundedHeight ? 1 : 0,
                        fit: FlexFit.tight,
                        child: frame,
                      ),
                    ],
                  ),
                );
              },
            );
          },
        );
      },
    );
  }
}
