import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/model/job/version_job.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/version_job_status.dart';

class VersionJobLines extends StatelessWidget {
  const VersionJobLines({super.key, required this.jobs, required this.detail});

  final List<VersionJob> jobs;
  final VersionDetail? detail;

  @override
  Widget build(BuildContext context) {
    final List<VersionJob> shown = subtitleWork(jobs);
    if (shown.isEmpty) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    final bool still = MediaQuery.disableAnimationsOf(context);
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (final VersionJob job in shown) _line(t, job, still),
      ],
    );
  }

  Widget _line(Tokens t, VersionJob job, bool still) {
    final String status = versionJobStatus(job, detail);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: Space.s1),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Expanded(
            child: Text(
              versionJobTitle(job),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(color: t.text, fontSize: 13),
            ),
          ),
          const SizedBox(width: Space.s2),
          Expanded(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Text(
                  status,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: _tone(t, job.status, status),
                    fontSize: 13,
                  ),
                ),
                if (job.status == JobStatus.running && !still)
                  Padding(
                    padding: const EdgeInsets.only(top: Space.s1),
                    child: ClipRRect(
                      borderRadius: const BorderRadius.all(Radii.pill),
                      child: LinearProgressIndicator(
                        minHeight: 3,
                        color: t.accent,
                        backgroundColor: t.surfaceAlt,
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Color _tone(Tokens t, JobStatus status, String text) => switch (status) {
    JobStatus.failed => t.danger,
    JobStatus.succeeded when text == Strings.jobNoSubtitle => t.warn,
    JobStatus.succeeded => t.ok,
    _ => t.muted,
  };
}
