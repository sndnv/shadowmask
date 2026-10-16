import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

Future<void> showPlaybackLogDialog(BuildContext context, List<String> lines) =>
    showDialog<void>(
      context: context,
      builder: (BuildContext _) => PlaybackLogDialog(lines: lines),
    );

class PlaybackLogDialog extends StatelessWidget {
  const PlaybackLogDialog({super.key, required this.lines});

  final List<String> lines;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return DialogShell(
      title: Strings.playerLog,
      width: 720,
      child: lines.isEmpty
          ? Text(Strings.playerLogEmpty, style: TextStyle(color: t.muted))
          : Text(
              lines.join('\n'),
              style: monoStyle.copyWith(
                color: t.text,
                fontSize: 12,
                height: 1.4,
              ),
            ),
    );
  }
}
