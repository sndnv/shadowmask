import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

Color? jobRowTint(BuildContext context, JobStatus status) {
  final Tokens t = context.tokens;
  return switch (status) {
    JobStatus.running => t.rowOk,
    JobStatus.queued => t.rowWarn,
    JobStatus.failed => t.rowDanger,
    _ => null,
  };
}

class JobStatusChip extends StatelessWidget {
  const JobStatusChip(this.status, {super.key});

  final JobStatus status;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final (Color fg, Color bg, String label) = switch (status) {
      JobStatus.queued => (t.warn, t.warnBg, Strings.statusQueued),
      JobStatus.running => (t.accent, t.okBg, Strings.statusRunning),
      JobStatus.succeeded => (t.ok, t.okBg, Strings.statusSucceeded),
      JobStatus.failed => (t.danger, t.dangerBg, Strings.statusFailed),
      JobStatus.cancelled => (t.muted, t.surfaceAlt, Strings.statusCancelled),
    };
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: Space.s2,
        vertical: Space.s1,
      ),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: const BorderRadius.all(Radii.sm),
      ),
      child: Text(
        label,
        style: Theme.of(context).textTheme.labelSmall?.copyWith(
          color: fg,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}
