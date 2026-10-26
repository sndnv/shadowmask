import 'package:flutter/widgets.dart';

import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/components/admin/library_busy_dialog.dart';
import 'package:shadowmask/components/admin/wipe_confirm_dialog.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/view/failure_reason.dart';

mixin LibraryMaintenance<T extends StatefulWidget> on Mutations<T> {
  Future<void> confirmMaintenance({
    required String title,
    required String message,
    required String confirmLabel,
    required String errorText,
    required String busyBody,
    required Future<String> Function(bool wipeUserData) run,
    required VoidCallback then,
    Future<void> Function()? cancelJobs,
  }) async {
    final bool? wipeUserData = await showWipeConfirmDialog(
      context,
      title: title,
      message: message,
      confirmLabel: confirmLabel,
    );
    if (wipeUserData == null || !mounted) {
      return;
    }
    List<Job>? blocking;
    await runBusy(() async {
      try {
        final String done = await run(wipeUserData);
        if (!mounted) {
          return;
        }
        Toasts.of(context).success(done);
        then();
      } catch (e) {
        if (!mounted) {
          return;
        }
        if (e is ApiException && e.code == 'library_busy') {
          blocking = busyJobsOf(e);
        } else {
          Toasts.of(context).error(failureText(errorText, e));
        }
      }
    });
    final List<Job>? jobs = blocking;
    if (jobs == null || !mounted) {
      return;
    }
    final bool cancel = await showLibraryBusyDialog(
      context,
      jobs,
      body: busyBody,
      canCancel: cancelJobs != null,
    );
    if (cancel) {
      await cancelJobs?.call();
    }
  }
}
