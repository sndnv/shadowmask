import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

Future<bool> showPlaybackLogDialog(
  BuildContext context,
  List<String> lines, {
  bool canReport = false,
}) async =>
    await showDialog<bool>(
      context: context,
      builder: (BuildContext _) =>
          PlaybackLogDialog(lines: lines, canReport: canReport),
    ) ??
    false;

class PlaybackLogDialog extends StatelessWidget {
  const PlaybackLogDialog({
    super.key,
    required this.lines,
    this.canReport = false,
  });

  final List<String> lines;
  final bool canReport;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return DialogShell(
      title: Strings.playerLog,
      width: 720,
      footer: canReport
          ? Align(
              alignment: Alignment.centerRight,
              child: FilledButton(
                onPressed: () => Navigator.of(context).pop(true),
                child: const Text(Strings.sendReport),
              ),
            )
          : null,
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
