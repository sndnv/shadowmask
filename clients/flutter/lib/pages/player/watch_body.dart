import 'dart:async';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/capability_scope.dart';
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
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/model/session/negotiation.dart';
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
import 'package:shadowmask/util/client_platform.dart';
import 'package:shadowmask/util/scoped_value.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shadowmask/view/episode_neighbours.dart';
import 'package:shadowmask/view/play_target.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/player_shortcuts.dart';
import 'package:shadowmask/view/track_carry.dart';

const int kStallSeconds = 20;

const int kPrebufferGiveUpSeconds = 10;

const int kBufferReachMs = 1000;
const int kSeekReachMs = 30000;

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
    this.touch = false,
  });

  final ApiClient api;
  final SelfUser user;
  final String versionId;
  final PlaybackControls initialControls;
  final PlayerControllerFactory controllerFactory;
  final bool wide;
  final bool touch;
  final VoidCallback? onToggleWide;

  @override
  State<WatchBody> createState() => _WatchBodyState();
}

class _WatchBodyState extends State<WatchBody>
    with RouteAware, WidgetsBindingObserver {
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
  int _originMs = 0;
  bool _sequential = false;
  TrickplayLoader? _trickplay;
  Timer? _heartbeat;
  String? _status;
  String? _mode;
  String? _container;
  String? _subtitleDelivery;
  String? _title;
  String? _detailRoute;
  List<Crumb> _crumbs = const <Crumb>[];
  String? _lastState;
  ScopedValue<bool>? _immersive;
  ClientDecoding? _decoding;
  int _autoplaySeconds = kDefaultPlayerPrefs.autoplaySeconds;
  int _networkTimeout = kDefaultPlayerPrefs.networkTimeoutSeconds;
  int _bufferSeconds = kDefaultPlayerPrefs.bufferSeconds;
  int _bufferBytes = kDefaultPlayerPrefs.bufferBytes;
  bool _waitForBuffer = kDefaultPlayerPrefs.waitForBuffer;
  bool _prebuffered = true;
  int _lastBufferMs = 0;
  int _bufferStalledSeconds = 0;
  String? _seriesId;
  EpisodeNeighbours _neighbours = const EpisodeNeighbours();
  EpisodeLink? _upNext;
  int _upNextLeft = 0;
  Timer? _countdown;
  bool _autoplayHandled = false;
  bool _carrySelection = false;
  Timer? _stallWatch;
  int _stalledSeconds = 0;
  int _lastProgressMs = -1;
  bool _stalled = false;
  int _negotiation = 0;
  final FocusNode _keys = FocusNode(debugLabel: 'player-shortcuts');

  @override
  void initState() {
    super.initState();
    _controls = widget.initialControls;
    _carrySelection = _controls.hasTrackRequest;
    _controller.snapshot.addListener(_onSnapshot);
    _controller.fullscreen.addListener(_onFullscreen);
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _immersive = ImmersiveScope.of(context);
    _decoding = CapabilityScope.of(context)?.decoding;
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
  void didPopNext() => _restart();

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (!widget.touch) {
      return;
    }
    if (state == AppLifecycleState.paused) {
      _release();
      return;
    }
    if (state == AppLifecycleState.resumed) {
      _restart(autoplay: false);
    }
  }

  void _restart({bool autoplay = true}) {
    if (_session != null ||
        !mounted ||
        !(ModalRoute.of(context)?.isCurrent ?? false)) {
      return;
    }
    setState(() {
      _boot = _start(autoplay: autoplay);
    });
  }

  void _release() {
    _heartbeat?.cancel();
    _heartbeat = null;
    _cancelCountdown();
    _controller.pause();
    _endSession();
    unawaited(_controller.detach());
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
    WidgetsBinding.instance.removeObserver(this);
    appRouteObserver.unsubscribe(this);
    _controller.fullscreen.removeListener(_onFullscreen);
    _immersive?.releaseAfterFrame(true, false, owner: this);
    _controller.snapshot.removeListener(_onSnapshot);
    _countdown?.cancel();
    _stallWatch?.cancel();
    _keys.dispose();
    _heartbeat?.cancel();
    _endSession();
    _trickplay?.dispose();
    _controller.dispose();
    super.dispose();
  }

  Future<bool> _start({bool autoplay = true}) async {
    try {
      final bool started = await _startSession(autoplay: autoplay);
      _immersive?.publish(_controller.fullscreen.value, owner: this);
      return started;
    } catch (_) {
      _immersive?.publish(false, owner: this);
      rethrow;
    }
  }

  void _retry() => setState(() => _boot = _start());

  Future<bool> _startSession({bool autoplay = true}) async {
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
      platform: clientPlatform(),
      decoding: _decoding,
      controls: _controls,
    );
    _session = session;
    _negotiation++;
    _version = version;
    _originMs = session.originMs;
    _sequential = session.sequential;
    _controls = _controls.withSelection(session.selected);
    _mode = session.mode.name;
    _container = session.container;
    _subtitleDelivery = session.selected?.subtitleDelivery;
    _trickplay = TrickplayLoader(
      api: _pb,
      versionId: widget.versionId,
      refs: session.trickplay.isNotEmpty
          ? session.trickplay
          : version.trickplay,
    );
    final PlayerPrefs prefs = await _prefs.load().catchError(
      (_) => kDefaultPlayerPrefs,
    );
    _networkTimeout = prefs.networkTimeoutSeconds;
    _controller.setNetworkTimeout(_networkTimeout);
    _bufferSeconds = prefs.bufferSeconds;
    _bufferBytes = prefs.bufferBytes;
    _waitForBuffer = prefs.waitForBuffer;
    _prebuffered = !(_waitForBuffer && autoplay);
    _lastBufferMs = 0;
    _bufferStalledSeconds = 0;
    _controller.setBuffer(seconds: _bufferSeconds, bytes: _bufferBytes);
    await _controller.attach(
      session.manifestUrl,
      mode: session.mode,
      positionMs: _attachOffset(resume.positionMs),
      autoplay: autoplay && _prebuffered,
    );
    if (!_prebuffered && !_controller.buffersAhead) {
      _prebuffered = true;
      _controller.play();
    }
    _remaining = prefs.remaining;
    _autoplaySeconds = prefs.autoplaySeconds;
    _controller.setVolume(prefs.volume);
    if (prefs.muted) {
      _controller.setMuted(true);
    }
    _startHeartbeat(session.heartbeatIntervalS);
    _watchForStall();
    if (!_unloadBound) {
      _unloadBound = true;
      addUnloadListener(_endSessionBeacon);
    }
    if (_controls.offsetMs != 0) {
      await _applyControls(
        _controls,
        seekMs: resume.positionMs,
        autoplay: autoplay,
      );
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

  void _watchForStall() {
    _stallWatch ??= Timer.periodic(const Duration(seconds: 1), (_) {
      if (!mounted) {
        return;
      }
      final PlayerSnapshot snap = _controller.snapshot.value;
      if (!_prebuffered) {
        final int ahead = snap.bufferedAheadMs;
        if (ahead + kBufferReachMs >= _prebufferTargetMs(snap)) {
          _playNow();
          return;
        }
        if (ahead > _lastBufferMs) {
          _bufferStalledSeconds = 0;
        } else if (++_bufferStalledSeconds >= kPrebufferGiveUpSeconds) {
          _playNow();
        }
        _lastBufferMs = ahead;
        return;
      }
      final int progress = snap.positionMs + snap.bufferedAheadMs;
      if (!snap.waiting || progress != _lastProgressMs) {
        _lastProgressMs = progress;
        _stalledSeconds = 0;
        if (_stalled) {
          setState(() => _stalled = false);
        }
        return;
      }
      _stalledSeconds += 1;
      if (_stalledSeconds >= kStallSeconds && !_stalled) {
        setState(() => _stalled = true);
      }
    });
  }

  void _keepWaiting() {
    _stalledSeconds = 0;
    setState(() => _stalled = false);
  }

  void _centrePlay() {
    if (_controller.snapshot.value.ended) {
      _cancelCountdown();
      _controller.seekTo(0);
    }
    _controller.togglePlay();
  }

  void _leavePlayer() {
    final NavigatorState navigator = Navigator.of(context);
    final String? route = _detailRoute;
    if (route != null) {
      navigator.pushReplacementNamed(route);
      return;
    }
    if (navigator.canPop()) {
      navigator.pop();
    }
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
        return watchRoute(target.id, controls: _carried());
      }
    } catch (_) {}
    return episodeRoute(link.id, series: _seriesId);
  }

  Map<String, String?> _carried() {
    final VersionDetail? version = _version;
    if (!_carrySelection || version == null) {
      return const <String, String?>{};
    }
    return carryTracks(_controls, version).toQuery();
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

  int get _fullDurationMs => _version?.durationMs ?? 0;

  int _attachOffset(int timelineMs) =>
      _sequential && !_controller.seeksWithinStream
      ? 0
      : timelineMs - _originMs;

  PlayerSnapshot get _view => _controller.snapshot.value.onTimeline(
    originMs: _originMs,
    fullMs: _fullDurationMs,
  );

  void _sendProgress(String state) {
    final String? sid = _session?.sessionId;
    if (sid != null) {
      _pb.progress(sid, _view.positionMs, state);
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

  Future<void> _applyControls(
    PlaybackControls next, {
    int? seekMs,
    bool autoplay = true,
  }) async {
    _carrySelection =
        _carrySelection ||
        next.audioTrack != _controls.audioTrack ||
        next.subtitle != _controls.subtitle ||
        next.subtitleOff != _controls.subtitleOff;
    setState(() => _controls = next);
    _writeUrl();
    final String? sid = _session?.sessionId;
    if (sid == null) {
      return;
    }
    final int keepMs = seekMs ?? _view.positionMs;
    final int negotiation = ++_negotiation;
    try {
      final neg = await _pb.update(sid, next);
      if (negotiation != _negotiation) {
        return;
      }
      _originMs = neg.originMs;
      _sequential = neg.sequential;
      await _controller.attach(
        neg.manifestUrl,
        mode: neg.mode,
        positionMs: _attachOffset(keepMs),
        autoplay: autoplay,
      );
      if (mounted && negotiation == _negotiation) {
        setState(() {
          _mode = neg.mode.name;
          _container = neg.container;
          _subtitleDelivery = neg.selected?.subtitleDelivery;
          _status = null;
        });
      }
    } catch (_) {
      if (mounted && negotiation == _negotiation) {
        setState(() => _status = Strings.renegotiationFailed);
      }
    }
  }

  void _changeSpeed(double rate) {
    setState(() => _speed = rate);
    _controller.setRate(rate);
  }

  void _holdSpeed(double? rate) => _controller.setRate(rate ?? _speed);

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

  void _changeNetworkTimeout(int seconds) {
    setState(() => _networkTimeout = seconds);
    _prefs.saveNetworkTimeoutSeconds(seconds);
    _controller.setNetworkTimeout(seconds);
  }

  bool get _canBuffer => _controller.buffersAhead;

  int _prebufferTargetMs(PlayerSnapshot snap) {
    final int chosen = _bufferSeconds * 1000;
    final int left = snap.durationMs - snap.positionMs;
    return left > 0 && left < chosen ? left : chosen;
  }

  void _changeBufferSeconds(int seconds) {
    setState(() => _bufferSeconds = seconds);
    _prefs.saveBufferSeconds(seconds);
    _controller.setBuffer(seconds: seconds, bytes: _bufferBytes);
  }

  void _changeBufferBytes(int bytes) {
    setState(() => _bufferBytes = bytes);
    _prefs.saveBufferBytes(bytes);
    _controller.setBuffer(seconds: _bufferSeconds, bytes: bytes);
  }

  void _changeWaitForBuffer(bool wait) {
    setState(() {
      _waitForBuffer = wait;
      if (!wait) {
        _prebuffered = true;
      }
    });
    _prefs.saveWaitForBuffer(wait);
  }

  void _playNow() {
    _lastBufferMs = 0;
    _bufferStalledSeconds = 0;
    setState(() => _prebuffered = true);
    _controller.play();
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
      case PlayerShortcut.back:
        _back();
    }
  }

  void _back() {
    if (_panel != null) {
      _closePanel();
      return;
    }
    if (_controller.fullscreen.value) {
      _controller.toggleFullscreen();
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
    final PlayerSnapshot s = _view;
    if (s.durationMs <= 0) {
      return;
    }
    final int target = toFraction != null
        ? (s.durationMs * toFraction).round()
        : s.positionMs + deltaMs!;
    _seekTo(target.clamp(0, s.durationMs));
  }

  bool _reachable(int targetMs) {
    if (!_sequential) {
      return true;
    }
    if (!_controller.seeksWithinStream) {
      return false;
    }
    if (targetMs < _originMs) {
      return false;
    }
    final PlayerSnapshot raw = _controller.snapshot.value;
    final int produced =
        _originMs + raw.positionMs + raw.bufferedAheadMs + kSeekReachMs;
    return targetMs <= produced;
  }

  void _seekTo(int targetMs) {
    if (_reachable(targetMs)) {
      _controller.seekTo(targetMs - _originMs);
      return;
    }
    unawaited(_restartAt(targetMs));
  }

  Future<void> _restartAt(int targetMs) async {
    final String? sid = _session?.sessionId;
    if (sid == null) {
      return;
    }
    final int negotiation = ++_negotiation;
    try {
      final Negotiation neg = await _pb.seek(sid, targetMs);
      if (negotiation != _negotiation) {
        return;
      }
      _originMs = neg.originMs;
      _sequential = neg.sequential;
      await _controller.attach(
        neg.manifestUrl,
        mode: neg.mode,
        positionMs: _attachOffset(targetMs),
      );
      if (mounted && negotiation == _negotiation) {
        setState(() {});
      }
    } catch (_) {}
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

  void _reclaimAfterPointer() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && !_typing) {
        _reclaimKeys();
      }
    });
  }

  void _reclaimKeys() {
    if (!mounted || _keys.hasPrimaryFocus) {
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
    if (widget.touch) {
      return;
    }
    _controller.togglePlay();
  }

  String? _waitingLabel(PlayerSnapshot snap) {
    if (!_prebuffered) {
      return Strings.playerBuffering;
    }
    if (!snap.waiting) {
      return null;
    }
    if (_stalled) {
      return Strings.playerTooSlow;
    }
    return snap.positionMs > 0
        ? Strings.playerBuffering
        : Strings.playerLoading;
  }

  String? _waitingDetail(PlayerSnapshot snap) {
    if (!_prebuffered) {
      return Strings.playerBufferingTo(
        snap.bufferedAheadMs / 1000,
        _prebufferTargetMs(snap) ~/ 1000,
      );
    }
    if (!snap.waiting) {
      return null;
    }
    if (_stalled) {
      return Strings.playerTooSlowHint;
    }
    if (snap.bufferedAheadMs > 0) {
      return Strings.playerBufferedSeconds(snap.bufferedAheadMs / 1000);
    }
    if (snap.bufferingPercent > 0) {
      return Strings.playerBufferedPercent(snap.bufferingPercent);
    }
    return null;
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
    child: Row(
      children: <Widget>[
        if (widget.touch)
          IconButton(
            onPressed: _leavePlayer,
            icon: const Icon(Icons.arrow_back),
            color: Colors.white,
            tooltip: Strings.playerGoBack,
          ),
        Expanded(
          child: Text(
            _title ?? '',
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(
              color: Colors.white,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
      ],
    ),
  );

  @override
  Widget build(BuildContext context) {
    return buildBlock<bool>(
      future: _boot,
      errorText: Strings.couldNotStartPlayback,
      onRetry: _retry,
      builder: (BuildContext context, bool _) {
        return ValueListenableBuilder<bool>(
          valueListenable: _controller.fullscreen,
          builder: (BuildContext context, bool fullscreen, _) {
            return ValueListenableBuilder<PlayerSnapshot>(
              valueListenable: _controller.snapshot,
              builder: (BuildContext context, PlayerSnapshot raw, _) {
                final PlayerSnapshot snap = raw.onTimeline(
                  originMs: _originMs,
                  fullMs: _fullDurationMs,
                );
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
                  onSeek: (double f) => _seekTo((f * snap.durationMs).round()),
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
                  child: Listener(
                    onPointerUp: (_) => _reclaimAfterPointer(),
                    child: PlayerFrame(
                      view: _controller.view,
                      playing: snap.playing,
                      fullscreen: fullscreen,
                      onTapVideo: _onTapVideo,
                      keepVisible: _panel != null,
                      status: snap.error ?? _status,
                      waiting: _waitingLabel(snap),
                      waitingDetail: _waitingDetail(snap),
                      onKeepWaiting: _stalled ? _keepWaiting : null,
                      onGoBack: _stalled ? _leavePlayer : null,
                      onPlayNow: _prebuffered ? null : _playNow,
                      ended: snap.ended,
                      onCentrePlay: _centrePlay,
                      touch: widget.touch,
                      onSeekRelative: _seekBy,
                      onHoldSpeed: _holdSpeed,
                      onDoubleTapVideo: widget.touch
                          ? null
                          : _controller.toggleFullscreen,
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
                                        container: _container,
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
                                        autoplaySeconds:
                                            _neighbours.next != null
                                            ? _autoplaySeconds
                                            : null,
                                        onAutoplaySeconds: _changeAutoplay,
                                        networkTimeoutSeconds: _canBuffer
                                            ? _networkTimeout
                                            : null,
                                        onNetworkTimeoutSeconds: _canBuffer
                                            ? _changeNetworkTimeout
                                            : null,
                                        bufferSeconds: _canBuffer
                                            ? _bufferSeconds
                                            : null,
                                        onBufferSeconds: _canBuffer
                                            ? _changeBufferSeconds
                                            : null,
                                        bufferBytes: _canBuffer
                                            ? _bufferBytes
                                            : null,
                                        onBufferBytes: _canBuffer
                                            ? _changeBufferBytes
                                            : null,
                                        waitForBuffer: _canBuffer
                                            ? _waitForBuffer
                                            : null,
                                        onWaitForBuffer: _canBuffer
                                            ? _changeWaitForBuffer
                                            : null,
                                        onShortcuts: widget.touch
                                            ? null
                                            : _showShortcuts,
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
                        builder:
                            (BuildContext context, bool muted, Widget? _) =>
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
                                        touch: widget.touch,
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
