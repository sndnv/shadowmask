import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/pages/player/watch_body.dart';
import 'package:shadowmask/view/playback_controls.dart';

class WatchPage extends StatefulWidget {
  const WatchPage({
    super.key,
    required this.api,
    this.controllerFactory = createPlayerController,
  });

  final ApiClient api;
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
    final PlayerPrefs prefs = await _prefs.load().catchError(
      (_) => kDefaultPlayerPrefs,
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
    final Map<String, String> query = Uri.base.queryParameters;
    final String versionId = query['version'] ?? '';
    if (versionId.isEmpty) {
      return SectionPage(
        api: widget.api,
        section: NavSection.none,
        bodyBuilder: (BuildContext context, SelfUser user) =>
            _centered(context, Strings.noVersionSpecified),
      );
    }
    final bool compact = compactViewport(context);
    final bool wide = _wide || compact;
    return SectionPage(
      api: widget.api,
      section: NavSection.none,
      errorText: Strings.couldNotStartPlayback,
      fullWidth: wide,
      fitViewport: wide,
      bodyBuilder: (BuildContext context, SelfUser user) =>
          SelectionContainer.disabled(
            child: WatchBody(
              api: widget.api,
              user: user,
              versionId: versionId,
              initialControls: PlaybackControls.fromQuery(query),
              controllerFactory: widget.controllerFactory,
              wide: wide,
              onToggleWide: compact ? null : _toggleWide,
            ),
          ),
    );
  }
}

Widget _centered(BuildContext context, String text) => Padding(
  padding: const EdgeInsets.all(Space.s5),
  child: Center(
    child: Text(text, style: TextStyle(color: context.tokens.muted)),
  ),
);
