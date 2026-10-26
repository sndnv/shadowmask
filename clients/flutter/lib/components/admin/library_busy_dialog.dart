import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/job_status_chip.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/job_labels.dart';

List<Job> busyJobsOf(ApiException error) => <Job>[
  for (final Map<String, dynamic> json in error.jobs)
    if (_readJob(json) case final Job job) job,
];

Job? _readJob(Map<String, dynamic> json) {
  try {
    return Job.fromJson(json);
  } catch (_) {
    return null;
  }
}

Future<bool> showLibraryBusyDialog(
  BuildContext context,
  List<Job> jobs, {
  required String body,
  bool canCancel = true,
}) async {
  final NavigatorState navigator = Navigator.of(context);
  final bool? cancel = await showDialog<bool>(
    context: context,
    barrierColor: Colors.black.withValues(alpha: 0.5),
    builder: (BuildContext ctx) {
      final Tokens t = ctx.tokens;
      return DialogShell(
        title: Strings.libraryBusy,
        onClose: () => Navigator.of(ctx).pop(false),
        footer: Wrap(
          alignment: WrapAlignment.end,
          spacing: Space.s3,
          runSpacing: Space.s2,
          children: <Widget>[
            OutlinedButton(
              onPressed: () => Navigator.of(ctx).pop(false),
              child: const Text(Strings.close),
            ),
            if (canCancel)
              FilledButton(
                onPressed: () => Navigator.of(ctx).pop(true),
                child: const Text(Strings.cancelThisLibrarysJobs),
              ),
          ],
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              body,
              style: Theme.of(
                ctx,
              ).textTheme.bodyMedium?.copyWith(color: t.text),
            ),
            const SizedBox(height: Space.s3),
            for (final Job job in jobs)
              Material(
                type: MaterialType.transparency,
                child: ListTile(
                  contentPadding: EdgeInsets.zero,
                  dense: true,
                  title: Text(jobKindLabel(job.kind)),
                  subtitle: Text(shortJobId(job.id)),
                  trailing: JobStatusChip(job.status),
                  onTap: () {
                    Navigator.of(ctx).pop(false);
                    navigator.pushNamed(jobRoute(job.id));
                  },
                ),
              ),
          ],
        ),
      );
    },
  );
  return cancel ?? false;
}
