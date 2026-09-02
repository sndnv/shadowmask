import 'dart:async';

import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_filter_field.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/danger_icon_button.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/job_status_chip.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/components/pagination.dart';
import 'package:shadowmask/components/progress_meter.dart';
import 'package:shadowmask/components/segmented_tabs.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/timestamp_text.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/model/job/jobs_feed.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/job_labels.dart';

enum _JobTab { active, all }

class JobsPage extends StatelessWidget {
  const JobsPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      loading: const SkeletonPage(child: SkeletonRows()),
      bodyBuilder: (BuildContext context, SelfUser user) => user.isAdmin
          ? _JobsBody(admin: AdminApi(api))
          : const StatusText(Strings.notAuthorized),
    );
  }
}

class _JobsBody extends StatefulWidget {
  const _JobsBody({required this.admin});

  final AdminApi admin;

  @override
  State<_JobsBody> createState() => _JobsBodyState();
}

class _JobsBodyState extends State<_JobsBody> with Mutations<_JobsBody> {
  late Future<JobsFeed> _future = _load();
  final TextEditingController _filter = TextEditingController();
  Timer? _debounce;
  _JobTab _tab = _JobTab.active;
  String _needle = '';
  int _offset = 0;
  int _activeTotal = 0;
  int _allTotal = 0;

  @override
  void dispose() {
    _debounce?.cancel();
    _filter.dispose();
    super.dispose();
  }

  Future<JobsFeed> _load() {
    final Future<JobsFeed> pending = widget.admin.jobs(
      offset: _offset,
      filter: _needle,
      activeOnly: _tab == _JobTab.active,
    );
    pending.then((JobsFeed feed) {
      if (!mounted ||
          !identical(_future, pending) ||
          (feed.activeTotal == _activeTotal && feed.allTotal == _allTotal)) {
        return;
      }
      setState(() {
        _activeTotal = feed.activeTotal;
        _allTotal = feed.allTotal;
      });
    }, onError: (Object _, StackTrace _) {});
    return pending;
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  void _goTo(int offset) {
    setState(() {
      _offset = offset;
      _future = _load();
    });
  }

  void _switchTab(_JobTab next) {
    setState(() {
      _tab = next;
      _offset = 0;
      _future = _load();
    });
  }

  void _onFilterChanged(String value) {
    _debounce?.cancel();
    _debounce = Timer(kFilterDebounce, () {
      if (!mounted) {
        return;
      }
      setState(() {
        _needle = value.trim();
        _offset = 0;
        _future = _load();
      });
    });
  }

  Future<void> _cancel(Job job) async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.cancelJob,
      message: Strings.confirmCancelJob(jobKindLabel(job.kind)),
      confirmLabel: Strings.cancelJob,
    );
    if (!ok) {
      return;
    }
    await mutate(
      key: job.id,
      () => widget.admin.cancelJob(job.id),
      successText: Strings.toastJobCancelled,
      errorText: Strings.errorAction,
      then: _reload,
    );
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Breadcrumbs(<Crumb>[
          Crumb(Strings.adminHeading, route: adminRoute()),
          Crumb(Strings.adminJobs),
        ]),
        PageActions(<PageAction>[
          PageAction(
            icon: Icons.refresh,
            label: Strings.refresh,
            onPressed: _reload,
          ),
        ]),
        const SizedBox(height: Space.s3),
        SegmentedTabs<_JobTab>(
          current: _tab,
          tabs: <(_JobTab, String)>[
            (_JobTab.active, Strings.jobsActiveCount(_activeTotal)),
            (_JobTab.all, Strings.jobsAllCount(_allTotal)),
          ],
          onChanged: _switchTab,
        ),
        const SizedBox(height: Space.s3),
        AdminFilterField(
          controller: _filter,
          hintText: Strings.filterJobs,
          onChanged: _onFilterChanged,
        ),
        const SizedBox(height: Space.s3),
        TabPanel(
          label: _tab == _JobTab.active
              ? Strings.jobsActiveCount(_activeTotal)
              : Strings.jobsAllCount(_allTotal),
          child: buildBlock<JobsFeed>(
            future: _future,
            errorText: Strings.couldNotLoadJobs,
            builder: (BuildContext context, JobsFeed data) => Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                _table(data.page.items),
                Pagination(
                  basePath: adminJobsRoute(),
                  params: const <String, String?>{},
                  total: data.page.total,
                  offset: data.page.offset,
                  limit: data.page.limit,
                  count: data.page.items.length,
                  onOffset: _goTo,
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }

  Widget _table(List<Job> jobs) => AdminTable<Job>(
    rows: jobs,
    emptyText: _emptyText(),
    onRowTap: (Job j) => Navigator.of(context).pushNamed(jobRoute(j.id)),
    rowLabel: Strings.openJob,
    rowColor: (Job j) => jobRowTint(context, j.status),
    minWidth: 1180,
    columns: <AdminColumn<Job>>[
      AdminColumn<Job>(
        label: Strings.columnJob,
        size: AdminColumnSize.small,
        cell: (BuildContext c, Job j) => _JobId(job: j),
      ),
      AdminColumn<Job>(
        label: Strings.columnParent,
        size: AdminColumnSize.small,
        cell: (BuildContext c, Job j) => _ParentCell(parentId: j.parentId),
      ),
      AdminColumn<Job>(
        label: Strings.columnKind,
        size: AdminColumnSize.large,
        essential: true,
        cell: (BuildContext c, Job j) =>
            Text(jobKindLabel(j.kind), overflow: TextOverflow.ellipsis),
      ),
      AdminColumn<Job>(
        label: Strings.columnStatus,
        essential: true,
        cell: (BuildContext c, Job j) => JobStatusChip(j.status),
      ),
      AdminColumn<Job>(
        label: Strings.columnPriority,
        size: AdminColumnSize.small,
        cell: (BuildContext c, Job j) => Text(jobPriorityLabel(j.priority)),
      ),
      AdminColumn<Job>(
        label: Strings.columnProgress,
        cell: (BuildContext c, Job j) =>
            ProgressMeter(percent: (j.progress * 100).round(), width: 70),
      ),
      AdminColumn<Job>(
        label: Strings.columnAttempts,
        size: AdminColumnSize.small,
        align: AdminColumnAlign.end,
        cell: (BuildContext c, Job j) => Text('${j.attempts}'),
      ),
      AdminColumn<Job>(
        label: Strings.columnStarted,
        cell: (BuildContext c, Job j) => TimestampText(j.startedAt),
      ),
      AdminColumn<Job>(
        label: Strings.columnFinished,
        cell: (BuildContext c, Job j) => TimestampText(j.finishedAt),
      ),
      AdminColumn<Job>(
        label: Strings.jobFactError,
        size: AdminColumnSize.small,
        cell: (BuildContext c, Job j) => _ErrorCell(job: j),
      ),
      AdminColumn<Job>(
        label: Strings.columnActions,
        size: AdminColumnSize.small,
        align: AdminColumnAlign.end,
        cell: (BuildContext c, Job j) => _cancelCell(j),
      ),
    ],
  );

  String _emptyText() {
    if (_needle.isNotEmpty) {
      return Strings.noMatchingJobs;
    }
    return _tab == _JobTab.active ? Strings.emptyActiveJobs : Strings.emptyJobs;
  }

  Widget _cancelCell(Job job) => job.cancellable
      ? DangerIconButton(
          icon: Icons.cancel_outlined,
          tooltip: Strings.cancelJob,
          onPressed: busy(job.id) ? null : () => _cancel(job),
        )
      : const SizedBox.shrink();
}

class _JobId extends StatelessWidget {
  const _JobId({required this.job});

  final Job job;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Text(
      shortJobId(job.id),
      overflow: TextOverflow.ellipsis,
      style: monoStyle.copyWith(color: t.text, fontSize: 12),
    );
  }
}

class _ParentCell extends StatelessWidget {
  const _ParentCell({required this.parentId});

  final String? parentId;

  @override
  Widget build(BuildContext context) {
    final String? id = parentId;
    if (id == null) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    return Align(
      alignment: Alignment.centerLeft,
      child: HoverTap(
        onTap: () => Navigator.of(context).pushNamed(jobRoute(id)),
        child: Text(
          shortJobId(id),
          overflow: TextOverflow.ellipsis,
          style: monoStyle.copyWith(
            color: t.accent,
            fontSize: 12,
            decoration: TextDecoration.underline,
            decorationColor: t.accent,
          ),
        ),
      ),
    );
  }
}

class _ErrorCell extends StatelessWidget {
  const _ErrorCell({required this.job});

  final Job job;

  static final RegExp _prefix = RegExp(
    r'^(retryable|permanent) job failure:\s*',
    caseSensitive: false,
  );

  @override
  Widget build(BuildContext context) {
    final String? raw = job.lastError;
    if (raw == null || raw.isEmpty) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    final String text = raw.replaceFirst(_prefix, '');
    final String label = text.length > 16 ? '${text.substring(0, 16)}…' : text;
    return Align(
      alignment: Alignment.centerLeft,
      child: HoverTap(
        onTap: () => showDialog<void>(
          context: context,
          builder: (BuildContext ctx) => DialogShell(
            title: Strings.jobErrorHeading,
            child: SelectableText(
              text,
              style: monoStyle.copyWith(color: ctx.tokens.text, fontSize: 12),
            ),
          ),
        ),
        semanticsLabel: Strings.jobErrorHeading,
        child: Text(
          label,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(
            color: t.danger,
            decoration: TextDecoration.underline,
            decorationColor: t.danger,
          ),
        ),
      ),
    );
  }
}
