import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/capability_scope.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/player/add_subtitles_dialog.dart';
import 'package:shadowmask/components/player/diagnostics_overlay.dart';
import 'package:shadowmask/components/player/overlay_bar.dart';
import 'package:shadowmask/components/player/playback_log_dialog.dart';
import 'package:shadowmask/components/player/player_frame.dart';
import 'package:shadowmask/components/player/player_sheet.dart';
import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/components/player/shortcuts_dialog.dart';
import 'package:shadowmask/components/player/timeline.dart';
import 'package:shadowmask/components/player/up_next_card.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/player/trickplay_loader.dart';
import 'package:shadowmask/components/player/trickplay_thumb.dart';
import 'package:shadowmask/player/playback_runner.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_life.dart';
import 'package:shadowmask/player/state/playback_notice.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_stream.dart';
import 'package:shadowmask/player/state/playback_title.dart';
import 'package:shadowmask/player/state/sent_change.dart';
import 'package:shadowmask/player/watches/buffer_watch.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/view/profile_version.dart';
import 'package:shadowmask/view/subtitle_size.dart';
import 'package:shadowmask/view/video_aspect.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/movie_detail.dart';
import 'package:shadowmask/model/catalog/season.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/server/server_info.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/model/session/resume_position.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
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
import 'package:shadowmask/view/version_jobs_watch.dart';

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
  late final PlayerController _controller;
  static const PlayerPrefsStore _prefs = PlayerPrefsStore();
  late Future<void> _opening = _open();
  late final PlaybackRunner _runner;
  late PlaybackState _shown;
  bool _gatherFailed = false;
  bool _covered = false;
  LocalSettings _saved = const LocalSettings();

  PlayerPanel? _panel;
  bool _sheetOpen = false;
  final GlobalKey<ScaffoldState> _sheetHost = GlobalKey<ScaffoldState>();
  TrickplayLoader? _trickplay;
  String? _trickplayFor;
  Artwork? _backdropArt;
  bool _claimedBackdrop = false;
  bool _ownsBackdrop = false;
  String? _detailRoute;
  List<Crumb> _crumbs = const <Crumb>[];
  String? _seriesId;
  ScopedValue<bool>? _immersive;
  ClientDecoding? _decoding;
  EpisodeLink? _upNext;
  int _upNextLeft = 0;
  Timer? _countdown;
  bool _upNextOff = false;
  bool _carrySelection = false;
  bool _warnedProfile = false;
  final FocusNode _keys = FocusNode(debugLabel: 'player-shortcuts');
  AdminApi? _admin;
  VersionJobsWatch? _jobs;
  final ValueNotifier<VersionDetail?> _versionFeed =
      ValueNotifier<VersionDetail?>(null);

  @override
  void initState() {
    super.initState();
    _controller = widget.controllerFactory(_pb.baseUrl);
    _runner = PlaybackRunner(
      api: _pb,
      player: _controller,
      saveLocal: _saveLocal,
      platform: clientPlatform(),
      versionId: widget.versionId,
      controls: widget.initialControls,
      playing: true,
    );
    _shown = _runner.state.value;
    _runner.state.addListener(_onState);
    addUnloadListener(_onUnload);
    addRestoreListener(_onRestore);
    _carrySelection = widget.initialControls.hasTrackRequest;
    WidgetsBinding.instance.addObserver(this);
    if (widget.user.worksOnVersions) {
      final AdminApi admin = AdminApi(widget.api);
      _admin = admin;
      _jobs = VersionJobsWatch(
        fetch: () => admin.versionJobs(widget.versionId),
        onFinished: _refetchVersion,
      );
    }
  }

  PlaybackState get _state => _runner.state.value;

  Future<bool> _refetchVersion() async {
    try {
      final VersionDetail version = await CatalogApi(
        widget.api,
      ).version(widget.versionId);
      if (mounted) {
        _runner.refreshVersion(version);
      }
      return true;
    } catch (_) {
      return false;
    }
  }

  void _onWork() {
    _jobs?.refresh();
    _refetchVersion();
  }

  Future<void> _addSubtitles() async {
    final AdminApi? admin = _admin;
    final VersionJobsWatch? jobs = _jobs;
    final PlaybackState s = _state;
    if (admin == null || jobs == null) {
      return;
    }
    jobs.refresh();
    final SubtitleSelection? watching = s.remote.subtitle.selection;
    await showAddSubtitlesDialog(
      context,
      admin: admin,
      version: _versionFeed,
      jobs: jobs,
      onWork: _onWork,
      audioTrack: s.remote.audioTrack,
      watchingFileId: watching?.kind == SubtitleKind.file ? watching?.id : null,
    );
    _reclaimKeys();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _immersive = ImmersiveScope.of(context);
    _decoding = CapabilityScope.of(context)?.decoding;
    if (!_claimedBackdrop) {
      _claimedBackdrop = true;
      _ownsBackdrop = BackdropScope.maybeOf(context)?.value == null;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _publishImmersive();
      }
    });
    final ModalRoute<dynamic>? route = ModalRoute.of(context);
    if (route is PageRoute<dynamic>) {
      appRouteObserver.subscribe(this, route);
    }
  }

  static bool _failed(PlaybackState s) =>
      s.stream == null && s.status.error != null;

  bool _immersed(PlaybackState s) =>
      !_gatherFailed && s.local.fullscreen && !_failed(s);

  void _publishImmersive() =>
      _immersive?.publish(_immersed(_state), owner: this);

  @override
  void didPushNext() {
    _covered = true;
    _background();
  }

  @override
  void didPopNext() {
    _covered = false;
    _foreground(playing: true);
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (!widget.touch) {
      return;
    }
    if (state == AppLifecycleState.paused) {
      _background();
      return;
    }
    if (state == AppLifecycleState.resumed) {
      _foreground(playing: false);
    }
  }

  void _background() {
    _cancelCountdown();
    _runner.background();
  }

  void _foreground({required bool playing}) {
    if (!mounted || _covered) {
      return;
    }
    _runner.foreground(playing: playing);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    appRouteObserver.unsubscribe(this);
    _immersive?.releaseAfterFrame(true, false, owner: this);
    _countdown?.cancel();
    _keys.dispose();
    _jobs?.dispose();
    _versionFeed.dispose();
    _trickplay?.dispose();
    _runner.state.removeListener(_onState);
    unawaited(_runner.leave().whenComplete(_controller.dispose));
    super.dispose();
  }

  void _warnProfileVersion(int reported) {
    if (_warnedProfile || profileVersionMatches(reported) || !mounted) {
      return;
    }
    _warnedProfile = true;
    Toasts.of(
      context,
    ).error(Strings.errorProfileVersion(reported, kExpectedProfileVersion));
  }

  Future<void> _open() async {
    try {
      await _gather();
    } catch (_) {
      if (mounted) {
        _gatherFailed = true;
        _publishImmersive();
      }
      rethrow;
    }
  }

  Future<void> _gather() async {
    final ServerInfo info = await _pb.serverInfo().catchError(
      (_) => const ServerInfo(),
    );
    _warnProfileVersion(info.profileVersion);
    final ResumePosition resume = await _pb
        .resume(widget.user.id, widget.versionId)
        .catchError((_) => const ResumePosition());
    final VersionDetail version = await CatalogApi(
      widget.api,
    ).version(widget.versionId);
    final LocalSettings prefs = await _prefs.load().catchError(
      (_) => const LocalSettings(),
    );
    if (!mounted) {
      return;
    }
    _saved = prefs;
    _runner.opened(
      title: PlaybackTitle(version: version),
      local: prefs.copyWith(decoding: _decoding),
      resumeMs: resume.positionMs,
      profileVersion: info.profileVersion,
    );
    _versionFeed.value = version;
    unawaited(_loadTitle(version.title));
  }

  void _reopen() {
    _gatherFailed = false;
    _publishImmersive();
    setState(() => _opening = _open());
  }

  Future<void> _saveLocal(LocalSettings local) {
    final LocalSettings was = _saved;
    _saved = local;
    return _prefs.saveChanged(was, local);
  }

  void _onUnload() => unawaited(_runner.leave());

  void _onRestore() {
    if (mounted && _state.status.life == PlaybackLife.closed) {
      reloadPage();
    }
  }

  void _onState() {
    final PlaybackState was = _shown;
    final PlaybackState now = _runner.state.value;
    _shown = now;
    if (!mounted) {
      return;
    }
    if (_immersed(now) != _immersed(was)) {
      _publishImmersive();
    }
    if (was.status.gathered &&
        was.status.sent is! SentStart &&
        now.remote != was.remote) {
      _carrySelection =
          _carrySelection ||
          now.remote.audioTrack != was.remote.audioTrack ||
          now.remote.subtitle != was.remote.subtitle;
      _writeUrl(now.remote);
    }
    final PlaybackNotice? notice = now.status.notice;
    if (notice != null && was.status.notice == null) {
      Toasts.of(context).error(switch (notice) {
        PlaybackNotice.changeFailed => Strings.renegotiationFailed,
        PlaybackNotice.seekFailed => Strings.seekFailed,
      });
    }
    if (now.title.version != was.title.version) {
      _versionFeed.value = now.title.version;
    }
    _followTrickplay(now);
    _followEnd(now);
  }

  void _followTrickplay(PlaybackState s) {
    final PlaybackStream? stream = s.stream;
    if (stream == null || stream.sessionId == _trickplayFor) {
      return;
    }
    _trickplayFor = stream.sessionId;
    _trickplay?.dispose();
    _trickplay = TrickplayLoader(
      api: _pb,
      versionId: widget.versionId,
      refs: stream.trickplay.isNotEmpty
          ? stream.trickplay
          : s.title.version?.trickplay ?? const <TrickplayRef>[],
    );
  }

  void _followEnd(PlaybackState s) {
    if (s.timeline.playing || !s.timeline.ended) {
      _upNextOff = false;
      _cancelCountdown();
      return;
    }
    if (s.status.stalled) {
      _upNextOff = true;
    }
    if (!s.watching || s.status.stalled || s.status.error != null) {
      _cancelCountdown();
      return;
    }
    _armAutoplay(s);
  }

  Future<void> _loadTitle(TitleRef ref) async {
    final CatalogApi catalog = CatalogApi(widget.api);
    if (ref.type == TitleKind.movie) {
      try {
        final MovieDetail movie = await catalog.movie(ref.id);
        if (!mounted) {
          return;
        }
        setState(() {
          _backdropArt = movie.artwork;
          _detailRoute = movieRoute(movie.id);
          _crumbs = <Crumb>[
            Crumb(Strings.navigationMovies, route: moviesRoute()),
            Crumb(movie.title, route: _detailRoute),
            const Crumb(Strings.watchHeading),
          ];
        });
        _runner.describe(
          name: movie.year != null
              ? Strings.titleWithYear(movie.title, movie.year!)
              : movie.title,
          previous: null,
          next: null,
        );
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
      final int? seasonNumber = season?.number ?? episode.seasonNumber;
      final EpisodeNeighbours neighbours = await loadNeighbours(
        catalog,
        episode,
      );
      if (!mounted) {
        return;
      }
      final String detail = episodeRoute(episode.id, series: seriesId);
      setState(() {
        _backdropArt = episode.seriesArtwork ?? episode.artwork;
        _seriesId = seriesId;
        _detailRoute = detail;
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
          Crumb(
            Strings.episodeCode(seasonNumber, episode.number),
            route: detail,
          ),
          const Crumb(Strings.watchHeading),
        ];
      });
      _runner.describe(
        name: Strings.seriesEpisodeLine(
          seriesTitle,
          seasonNumber,
          episode.number,
          episode.title,
        ),
        previous: neighbours.previous,
        next: neighbours.next,
      );
    } catch (_) {}
  }

  void _togglePlay() => _runner.toggle();

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

  void _armAutoplay(PlaybackState s) {
    final EpisodeLink? next = s.title.next;
    final int seconds = s.local.autoplayNextS;
    if (_upNextOff ||
        _upNext != null ||
        seconds == kAutoplayOff ||
        next == null) {
      return;
    }
    if (seconds == kAutoplayImmediately) {
      _goToEpisode(next);
      return;
    }
    setState(() {
      _upNext = next;
      _upNextLeft = seconds;
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

  void _dismissUpNext() {
    _upNextOff = true;
    _cancelCountdown();
  }

  Future<void> _goToEpisode(EpisodeLink link) async {
    _dismissUpNext();
    final String route = await _episodeRoute(link);
    if (!mounted) {
      return;
    }
    final NavigatorState nav = Navigator.of(context);
    final ModalRoute<dynamic>? page = ModalRoute.of(context);
    if (page != null && !page.isCurrent) {
      nav.popUntil((Route<dynamic> r) => r == page || r is! PopupRoute);
      if (!page.isCurrent) {
        return;
      }
    }
    await nav.pushReplacementNamed(route);
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
    final PlaybackState s = _state;
    final VersionDetail? version = s.title.version;
    if (!_carrySelection || version == null) {
      return const <String, String?>{};
    }
    return carryTracks(s.remote, version).toQuery();
  }

  TransportStep? _step(
    PlaybackTitle title,
    EpisodeLink? link,
    String none,
    String Function(String) named,
  ) {
    if (title.previous == null && title.next == null) {
      return null;
    }
    return (
      tooltip: link == null ? none : named(link.label),
      onPressed: link == null ? null : () => _goToEpisode(link),
    );
  }

  void _changeRemote(PlaybackControls next) => _runner.changeRemote(next);

  void _writeUrl(PlaybackControls controls) => replaceUrl(
    withQuery('/watch', <String, String?>{
      'version': widget.versionId,
      ...controls.toQuery(),
    }),
  );

  void _changeLocal(LocalSettings Function(LocalSettings local) change) =>
      _runner.changeLocal(change);

  void _holdSpeed(double? rate) =>
      _controller.setRate(rate ?? _state.local.speed);

  void _openPanel(PlayerPanel panel) {
    if (_state.title.version == null) {
      return;
    }
    setState(() => _panel = _panel == panel ? null : panel);
  }

  void _closePanel() {
    setState(() => _panel = null);
    final ScaffoldState? host = _sheetHost.currentState;
    if (host != null && host.isEndDrawerOpen) {
      host.closeEndDrawer();
    }
    _reclaimKeys();
  }

  void _openSheet() {
    if (_state.title.version == null) {
      return;
    }
    setState(() => _panel = null);
    _sheetHost.currentState?.openEndDrawer();
  }

  void _onSheetChanged(bool open) {
    if (!mounted) {
      return;
    }
    setState(() => _sheetOpen = open);
    if (!open) {
      _reclaimKeys();
    }
  }

  bool get _canBuffer => _controller.buffersAhead;

  void _changeAutoplay(int seconds) {
    _changeLocal((LocalSettings l) => l.copyWith(autoplayNextS: seconds));
    final EpisodeLink? next = _upNext;
    if (seconds == kAutoplayOff) {
      _cancelCountdown();
    } else if (seconds == kAutoplayImmediately && next != null) {
      _cancelCountdown();
      _goToEpisode(next);
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
        _togglePlay();
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
        _stepEpisode(_state.title.previous);
      case PlayerShortcut.nextEpisode:
        _stepEpisode(_state.title.next);
      case PlayerShortcut.remaining:
        _changeLocal((LocalSettings l) => l.copyWith(remaining: !l.remaining));
      case PlayerShortcut.legend:
        _showShortcuts();
      case PlayerShortcut.back:
        _back();
    }
  }

  void _back() {
    if (_panel != null || _sheetOpen) {
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

  Future<void> _showPlaybackLog() async {
    _closePanel();
    await showPlaybackLogDialog(context, _controller.log());
    _reclaimKeys();
  }

  void _stepEpisode(EpisodeLink? link) {
    if (link != null) {
      _goToEpisode(link);
    }
  }

  void _seekBy(int? deltaMs, {double? toFraction}) => toFraction != null
      ? _runner.seekToFraction(toFraction)
      : _runner.seekBy(deltaMs!);

  void _nudgeVolume(double by) => _changeLocal(
    (LocalSettings l) => _atVolume(l, (l.volume + by).clamp(0.0, 1.0)),
  );

  void _changeVolume(double level) =>
      _changeLocal((LocalSettings l) => _atVolume(l, level));

  static LocalSettings _atVolume(LocalSettings l, double level) =>
      l.copyWith(volume: level, muted: level == 0 && l.muted);

  void _toggleMute() {
    if (_controller.mutedByPolicy.value) {
      _controller.unmute();
      return;
    }
    _changeLocal((LocalSettings l) => l.copyWith(muted: !l.muted));
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
    if (_controller.mutedByPolicy.value) {
      _controller.unmute();
      return;
    }
    if (widget.touch) {
      return;
    }
    _togglePlay();
  }

  static PlayerSnapshot _shownReading(PlaybackState s) => PlayerSnapshot(
    positionMs: s.timeline.positionMs,
    durationMs: s.title.version?.durationMs ?? 0,
    bufferedAheadMs: s.timeline.bufferedAheadMs,
    playing: s.timeline.playing,
    ready: true,
    buffering: s.timeline.waiting,
    bufferingPercent: s.timeline.bufferingPercent,
    ended: s.timeline.ended,
  );

  String? _waitingLabel(PlaybackState s, PlayerSnapshot snap) {
    final SentChange? sent = s.status.sent;
    if (sent is SentControls) {
      return Strings.applyingPlaybackChanges;
    }
    if (sent != null) {
      return snap.positionMs > 0
          ? Strings.playerBuffering
          : Strings.playerLoading;
    }
    if (s.status.holding) {
      return Strings.playerBuffering;
    }
    if (s.status.stalled && s.status.error != null) {
      return Strings.playerFailed;
    }
    if (!snap.waiting) {
      return null;
    }
    if (s.status.stalled) {
      return Strings.playerTooSlow;
    }
    return snap.positionMs > 0
        ? Strings.playerBuffering
        : Strings.playerLoading;
  }

  String? _waitingDetail(PlaybackState s, PlayerSnapshot snap) {
    if (s.status.sent != null) {
      return null;
    }
    if (s.status.holding) {
      return Strings.playerBufferingTo(
        snap.bufferedAheadMs / 1000,
        bufferTargetMs(snap, s.local.bufferS) ~/ 1000,
      );
    }
    if (s.status.stalled && s.status.error != null) {
      return s.status.error;
    }
    if (!snap.waiting) {
      return null;
    }
    if (s.status.stalled) {
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

  String? _sourceLine(VersionDetail? version) {
    if (version == null || version.video.isEmpty) {
      return null;
    }
    final VideoTrack v = version.video.first;
    return 'source: ${v.codec} ${v.width}x${v.height} '
        '@ ${v.frameRate.toStringAsFixed(0)}fps';
  }

  Widget _titleRow(String? name) => Padding(
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
            name ?? '',
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
  Widget build(BuildContext context) => buildBlock<void>(
    future: _opening,
    errorText: Strings.couldNotStartPlayback,
    onRetry: _reopen,
    builder: (BuildContext context, void _) =>
        ValueListenableBuilder<PlaybackState>(
          valueListenable: _runner.state,
          builder: (BuildContext context, PlaybackState s, Widget? _) =>
              s.stream == null ? _waiting(context, s) : _page(context, s),
        ),
  );

  Widget _waiting(BuildContext context, PlaybackState s) {
    final String? error = s.status.error;
    return buildSnapshot<void>(
      context,
      error == null
          ? const AsyncSnapshot<void>.waiting()
          : AsyncSnapshot<void>.withError(ConnectionState.done, error),
      builder: (BuildContext context, void _) => const SizedBox.shrink(),
      errorText: error,
      onRetry: _runner.retry,
    );
  }

  Widget _settingsPanel(
    BuildContext context,
    PlaybackState s,
    PlayerPanel panel,
    VersionDetail version, {
    ChoiceOpener? openChoice,
  }) => PlayerSettingsPanel(
    panel: panel,
    controls: s.remote,
    version: version,
    speed: s.local.speed,
    diagnostics: s.local.diagnostics,
    mode: s.stream?.mode.name,
    container: s.stream?.container,
    subtitleDelivery: s.stream?.selected?.subtitleDelivery,
    pictureInPicture: _controller.supportsPictureInPicture
        ? s.local.pictureInPicture
        : null,
    onPictureInPicture: _controller.togglePictureInPicture,
    busy: s.status.sent != null,
    onControls: _changeRemote,
    onSpeed: (double rate) =>
        _changeLocal((LocalSettings l) => l.copyWith(speed: rate)),
    onDiagnostics: (bool on) =>
        _changeLocal((LocalSettings l) => l.copyWith(diagnostics: on)),
    autoplaySeconds: s.title.next != null ? s.local.autoplayNextS : null,
    onAutoplaySeconds: _changeAutoplay,
    subtitleSize: s.local.subtitleSize,
    onSubtitleSize: (SubtitleSize size) =>
        _changeLocal((LocalSettings l) => l.copyWith(subtitleSize: size)),
    networkTimeoutSeconds: _canBuffer ? s.local.networkTimeoutS : null,
    onNetworkTimeoutSeconds: _canBuffer
        ? (int seconds) => _changeLocal(
            (LocalSettings l) => l.copyWith(networkTimeoutS: seconds),
          )
        : null,
    bufferSeconds: _canBuffer ? s.local.bufferS : null,
    onBufferSeconds: _canBuffer
        ? (int seconds) =>
              _changeLocal((LocalSettings l) => l.copyWith(bufferS: seconds))
        : null,
    bufferBytes: _canBuffer ? s.local.bufferBytes : null,
    onBufferBytes: _canBuffer
        ? (int bytes) =>
              _changeLocal((LocalSettings l) => l.copyWith(bufferBytes: bytes))
        : null,
    waitForBuffer: _canBuffer ? s.local.waitForBuffer : null,
    onWaitForBuffer: _canBuffer
        ? (bool wait) =>
              _changeLocal((LocalSettings l) => l.copyWith(waitForBuffer: wait))
        : null,
    onShortcuts: widget.touch ? null : _showShortcuts,
    onShowLog: _showPlaybackLog,
    onAddSubtitles: _admin == null ? null : _addSubtitles,
    openChoice: openChoice,
    onClose: _closePanel,
    dense: compactViewport(context),
  );

  Widget _page(BuildContext context, PlaybackState s) {
    final PlayerSnapshot snap = _shownReading(s);
    final bool fullscreen = s.local.fullscreen;
    final bool blocked = !s.watching;
    final VersionDetail? version = s.title.version;
    final double buffered = snap.durationMs > 0
        ? ((snap.positionMs + snap.bufferedAheadMs) / snap.durationMs).clamp(
            0.0,
            1.0,
          )
        : 0.0;
    final TrickplayLoader? tp = _trickplay;
    final Timeline timeline = Timeline(
      fraction: snap.fraction,
      bufferedFraction: buffered,
      durationMs: snap.durationMs,
      onSeek: blocked ? null : _runner.seekToFraction,
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
    final bool stalled = s.status.stalled && s.status.sent == null;
    final Widget frame = Focus(
      autofocus: true,
      focusNode: _keys,
      onKeyEvent: _onKey,
      child: Listener(
        onPointerUp: (_) => _reclaimAfterPointer(),
        child: PlayerFrame(
          view: _controller.view,
          videoAspect: videoAspect(version),
          playing: snap.playing,
          fullscreen: fullscreen,
          onTapVideo: _onTapVideo,
          keepVisible: _panel != null || _sheetOpen,
          waiting: _waitingLabel(s, snap),
          waitingDetail: _waitingDetail(s, snap),
          keepWaitingLabel: s.status.error == null
              ? Strings.playerKeepWaiting
              : Strings.dismiss,
          onKeepWaiting: stalled ? _runner.keepWaiting : null,
          onGoBack: stalled || s.status.slow ? _leavePlayer : null,
          onPlayNow: s.status.holding ? _runner.play : null,
          ended: snap.ended,
          onCentrePlay: _togglePlay,
          touch: widget.touch,
          onSeekRelative: _seekBy,
          onHoldSpeed: _holdSpeed,
          onDoubleTapVideo: widget.touch ? null : _controller.toggleFullscreen,
          diagnostics: s.local.diagnostics
              ? DiagnosticsOverlay(
                  controller: _controller,
                  sourceLine: _sourceLine(version),
                )
              : null,
          settings: _panel != null && version != null
              ? _settingsPanel(context, s, _panel!, version)
              : null,
          sheet: widget.touch && version != null
              ? PlayerSheet(
                  section: (PlayerPanel panel, ChoiceOpener open) =>
                      _settingsPanel(
                        context,
                        s,
                        panel,
                        version,
                        openChoice: open,
                      ),
                  summary: (PlayerPanel panel) =>
                      panelSummary(panel, s.remote, version),
                  onClose: _closePanel,
                )
              : null,
          sheetHost: _sheetHost,
          onSheetChanged: _onSheetChanged,
          onDismissSettings: _closePanel,
          upNext: _upNext == null
              ? null
              : UpNextCard(
                  label: _upNext!.label,
                  remainingSeconds: _upNextLeft,
                  onPlay: () => _goToEpisode(_upNext!),
                  onCancel: _dismissUpNext,
                ),
          back: _titleRow(s.title.name),
          overlay: OverlayBar(
            snapshot: snap,
            timeline: timeline,
            fullscreen: fullscreen,
            wide: widget.wide,
            touch: widget.touch,
            muted: s.local.muted || s.local.mutedByPolicy,
            volume: s.local.volume,
            remaining: s.local.remaining,
            onPlayPause: blocked ? null : _togglePlay,
            onOpenPanel: _openPanel,
            onOpenSheet: widget.touch ? _openSheet : null,
            onToggleFullscreen: _controller.toggleFullscreen,
            onToggleWide: widget.onToggleWide,
            onToggleMute: _toggleMute,
            onVolume: _changeVolume,
            onToggleRemaining: () => _changeLocal(
              (LocalSettings l) => l.copyWith(remaining: !l.remaining),
            ),
            previousEpisode: _step(
              s.title,
              s.title.previous,
              Strings.noEarlierEpisode,
              Strings.previousNamed,
            ),
            nextEpisode: _step(
              s.title,
              s.title.next,
              Strings.noLaterEpisode,
              Strings.nextNamed,
            ),
          ),
        ),
      ),
    );
    BackdropScope.keep(context);
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints c) => Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          if (!fullscreen) Breadcrumbs(_crumbs),
          Flexible(
            flex: c.hasBoundedHeight ? 1 : 0,
            fit: fullscreen ? FlexFit.tight : FlexFit.loose,
            child: frame,
          ),
          if (_ownsBackdrop && _backdropArt != null)
            PageBackdrop(
              artwork: _backdropArt,
              imageBase: CatalogApi(widget.api).imageBase,
            ),
        ],
      ),
    );
  }
}
