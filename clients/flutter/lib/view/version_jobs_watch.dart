import 'dart:async';

import 'package:flutter/foundation.dart';

import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/model/job/version_job.dart';
import 'package:shadowmask/view/version_job_status.dart';

const Duration kJobsBusyPoll = Duration(seconds: 5);

bool awaitedWork(VersionJob job) =>
    kSubtitleWorkKinds.contains(job.kind) &&
    (job.status == JobStatus.running ||
        (job.status == JobStatus.queued && job.ahead != null));

class VersionJobsWatch extends ValueNotifier<List<VersionJob>> {
  VersionJobsWatch({required this.fetch, required this.onFinished})
    : super(const <VersionJob>[]);

  final Future<List<VersionJob>> Function() fetch;
  final Future<bool> Function() onFinished;
  Timer? _timer;
  bool _loaded = false;
  bool _closed = false;
  int _generation = 0;

  void refresh() {
    _timer?.cancel();
    _poll();
  }

  Future<void> _poll() async {
    final int generation = ++_generation;
    List<VersionJob>? rows;
    try {
      rows = await fetch();
    } catch (_) {}
    if (_closed || generation != _generation) {
      return;
    }
    bool settled = true;
    if (rows != null) {
      if (_loaded && rows.any(_justFinished)) {
        settled = await _reread();
        if (_closed || generation != _generation) {
          return;
        }
      }
      if (settled) {
        _loaded = true;
        value = rows;
      }
    }
    if (!settled || value.any(awaitedWork)) {
      _timer = Timer(kJobsBusyPoll, _poll);
    }
  }

  Future<bool> _reread() async {
    try {
      return await onFinished();
    } catch (_) {
      return false;
    }
  }

  bool _justFinished(VersionJob job) =>
      kSubtitleWorkKinds.contains(job.kind) && !job.active && _wasOpen(job);

  bool _wasOpen(VersionJob job) {
    for (final VersionJob before in value) {
      if (before.id == job.id) {
        return before.active;
      }
    }
    return true;
  }

  @override
  void dispose() {
    _closed = true;
    _timer?.cancel();
    super.dispose();
  }
}
