import 'package:flutter/material.dart';

import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class PlaybackBlock extends StatefulWidget {
  const PlaybackBlock({super.key, this.store = const PlayerPrefsStore()});

  final PlayerPrefsStore store;

  @override
  State<PlaybackBlock> createState() => _PlaybackBlockState();
}

class _PlaybackBlockState extends State<PlaybackBlock> {
  int _autoplaySeconds = kDefaultPlayerPrefs.autoplaySeconds;
  bool _diagnostics = kDefaultPlayerPrefs.diagnostics;
  bool _loaded = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final PlayerPrefs prefs = await widget.store.load().catchError(
      (Object _) => kDefaultPlayerPrefs,
    );
    if (!mounted) {
      return;
    }
    setState(() {
      _autoplaySeconds = prefs.autoplaySeconds;
      _diagnostics = prefs.diagnostics;
      _loaded = true;
    });
  }

  void _setAutoplay(int seconds) {
    setState(() => _autoplaySeconds = seconds);
    widget.store.saveAutoplaySeconds(seconds);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  void _setDiagnostics(bool on) {
    setState(() => _diagnostics = on);
    widget.store.saveDiagnostics(on);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return SectionBlock(
      title: Strings.accountPlaybackHeading,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Material(
            color: Colors.transparent,
            child: ListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text(
                Strings.playerAutoplayNext,
                style: TextStyle(fontWeight: FontWeight.w600),
              ),
              subtitle: Text(
                Strings.playerAutoplayNextHelp,
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: t.muted),
              ),
              trailing: AppDropdown<int>(
                value: _autoplaySeconds,
                label: Strings.playerAutoplayNext,
                enabled: _loaded,
                items: <(int, String)>[
                  for (final int seconds in kAutoplayDelays)
                    (
                      seconds,
                      seconds == 0
                          ? Strings.playerAutoplayOff
                          : Strings.playerAutoplayDelay(seconds),
                    ),
                ],
                onChanged: _setAutoplay,
              ),
            ),
          ),
          Divider(color: t.border, height: 1),
          Material(
            color: Colors.transparent,
            child: SwitchListTile(
              contentPadding: EdgeInsets.zero,
              value: _diagnostics,
              onChanged: _loaded ? _setDiagnostics : null,
              title: const Text(
                Strings.playerDiagnostics,
                style: TextStyle(fontWeight: FontWeight.w600),
              ),
              subtitle: Text(
                Strings.playerDiagnosticsHelp,
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: t.muted),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
