import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playlist_api.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/playlist/queue_entry.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/view/play_route.dart';
import 'package:shadowmask/view/queue_next.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/touch_platform.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/pages/player/watch_body.dart';
import 'package:shadowmask/view/playback_controls.dart';

class WatchPage extends StatefulWidget {
  const WatchPage({
    super.key,
    required this.api,
    required this.versionId,
    this.controls = const PlaybackControls(),
    this.startAtMs,
    this.queueEntryId,
    this.controllerFactory = createPlayerController,
  });

  final ApiClient api;
  final String versionId;
  final PlaybackControls controls;
  final int? startAtMs;
  final String? queueEntryId;
  final PlayerControllerFactory controllerFactory;

  @override
  State<WatchPage> createState() => _WatchPageState();
}

class _WatchPageState extends State<WatchPage> {
  static const PlayerPrefsStore _prefs = PlayerPrefsStore();
  bool _wide = false;

  @override
  void initState() {
    super.initState();
    _restoreWide();
  }

  Future<void> _restoreWide() async {
    final LocalSettings prefs = await _prefs.load().catchError(
      (_) => const LocalSettings(),
    );
    if (mounted && prefs.wide != _wide) {
      setState(() => _wide = prefs.wide);
    }
  }

  void _toggleWide() {
    setState(() => _wide = !_wide);
    _prefs.saveWide(_wide);
  }

  @override
  Widget build(BuildContext context) {
    final String versionId = widget.versionId;
    final String? queued = widget.queueEntryId;
    if (versionId.isEmpty) {
      return SectionPage(
        api: widget.api,
        section: NavSection.none,
        bodyBuilder: (BuildContext context, SelfUser user) => queued == null
            ? _centered(context, Strings.noVersionSpecified)
            : _QueueStart(api: widget.api, user: user, entryId: queued),
      );
    }
    final bool compact = compactViewport(context);
    final bool touch = touchPlatform(context);
    final bool wide = _wide || compact;
    return SectionPage(
      api: widget.api,
      section: NavSection.none,
      errorText: Strings.couldNotStartPlayback,
      fullWidth: wide,
      fitViewport: true,
      keepsBackdrop: true,
      bodyBuilder: (BuildContext context, SelfUser user) =>
          SelectionContainer.disabled(
            child: WatchBody(
              api: widget.api,
              user: user,
              versionId: versionId,
              initialControls: widget.controls,
              startAtMs: widget.startAtMs,
              queueEntryId: queued,
              controllerFactory: widget.controllerFactory,
              wide: wide,
              touch: touch,
              onToggleWide: compact ? null : _toggleWide,
            ),
          ),
    );
  }
}

class _QueueStart extends StatefulWidget {
  const _QueueStart({
    required this.api,
    required this.user,
    required this.entryId,
  });

  final ApiClient api;
  final SelfUser user;
  final String entryId;

  @override
  State<_QueueStart> createState() => _QueueStartState();
}

class _QueueStartState extends State<_QueueStart> {
  @override
  void initState() {
    super.initState();
    _start();
  }

  Future<void> _start() async {
    String route = playlistsRoute();
    try {
      final List<QueueEntry> queue = await PlaylistApi(
        widget.api,
      ).queue(widget.user.id);
      final QueueEntry? entry = queue
          .where((QueueEntry e) => e.id == widget.entryId)
          .firstOrNull;
      if (entry != null) {
        route = await playRouteFor(
          CatalogApi(widget.api),
          widget.user.id,
          queueLink(entry),
        );
      }
    } catch (_) {}
    if (mounted) {
      await Navigator.of(context).pushReplacementNamed(route);
    }
  }

  @override
  Widget build(BuildContext context) => const Center(
    child: Padding(
      padding: EdgeInsets.all(Space.s5),
      child: CircularProgressIndicator(strokeWidth: 2),
    ),
  );
}

Widget _centered(BuildContext context, String text) => Padding(
  padding: const EdgeInsets.all(Space.s5),
  child: Center(
    child: Text(text, style: TextStyle(color: context.tokens.muted)),
  ),
);
