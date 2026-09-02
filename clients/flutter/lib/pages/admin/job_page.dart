import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/job_children.dart';
import 'package:shadowmask/components/admin/job_status_chip.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/timestamp_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/model/job/job_node.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/job_labels.dart';

const String kDefaultLogLevel = 'INFO';

class JobPage extends StatelessWidget {
  const JobPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    final String? id = Uri.base.queryParameters['id'];
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      bodyBuilder: (BuildContext context, SelfUser user) {
        if (!user.isAdmin) {
          return const StatusText(Strings.notAuthorized);
        }
        if (id == null || id.isEmpty) {
          return const StatusText(Strings.couldNotLoadJob);
        }
        return _JobBody(admin: AdminApi(api), id: id);
      },
    );
  }
}

class _JobBody extends StatefulWidget {
  const _JobBody({required this.admin, required this.id});

  final AdminApi admin;
  final String id;

  @override
  State<_JobBody> createState() => _JobBodyState();
}

class _JobBodyState extends State<_JobBody> with Mutations<_JobBody> {
  String _level = kDefaultLogLevel;
  late Future<Job> _future = widget.admin.job(widget.id);
  late Future<List<String>> _logs = widget.admin.jobLog(
    widget.id,
    level: _level,
  );
  int _childOffset = 0;
  late Future<Paged<JobNode>> _children = widget.admin.jobChildren(widget.id);

  void _reload() {
    setState(() {
      _future = widget.admin.job(widget.id);
      _logs = widget.admin.jobLog(widget.id, level: _level);
      _children = widget.admin.jobChildren(widget.id, offset: _childOffset);
    });
  }

  void _goToChildren(int offset) {
    setState(() {
      _childOffset = offset;
      _children = widget.admin.jobChildren(widget.id, offset: offset);
    });
  }

  void _setLevel(String level) {
    setState(() {
      _level = level;
      _logs = widget.admin.jobLog(widget.id, level: level);
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
      () => widget.admin.cancelJob(job.id),
      successText: Strings.toastJobCancelled,
      errorText: Strings.errorAction,
      then: _reload,
    );
  }

  Future<void> _wipe() async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.wipeLogs,
      message: Strings.confirmWipeLogs,
      confirmLabel: Strings.wipeLogs,
    );
    if (!ok) {
      return;
    }
    await mutate(
      () => widget.admin.wipeJobLog(widget.id),
      successText: Strings.toastLogsWiped,
      errorText: Strings.errorAction,
      then: () => setState(() {
        _logs = widget.admin.jobLog(widget.id);
      }),
    );
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<Job>(
      future: _future,
      errorText: Strings.couldNotLoadJob,
      builder: (BuildContext context, Job job) => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Breadcrumbs(<Crumb>[
            Crumb(Strings.adminHeading, route: adminRoute()),
            Crumb(Strings.adminJobs, route: adminJobsRoute()),
            Crumb(jobKindLabel(job.kind)),
          ]),
          PageActions(<PageAction>[
            if (job.cancellable)
              PageAction(
                icon: Icons.cancel_outlined,
                label: Strings.cancelJob,
                danger: true,
                onPressed: busy() ? null : () => _cancel(job),
              ),
            PageAction(
              icon: Icons.refresh,
              label: Strings.refresh,
              onPressed: _reload,
            ),
          ]),
          const SizedBox(height: Space.s4),
          SectionBlock(
            title: job.id,
            child: _Facts(job: job),
          ),
          SectionBlock(
            title: Strings.jobLogsHeading,
            actions: <Widget>[
              AppDropdown<String>(
                value: _level,
                width: 130,
                label: Strings.jobLogsHeading,
                items: const <(String, String)>[
                  ('DEBUG', 'Debug'),
                  ('INFO', 'Info'),
                  ('WARN', 'Warn'),
                  ('ERROR', 'Error'),
                ],
                onChanged: _setLevel,
              ),
            ],
            actionItems: <PageAction>[
              PageAction(
                icon: Icons.delete_outline,
                label: Strings.wipeLogs,
                danger: true,
                onPressed: busy() ? null : _wipe,
              ),
            ],
            child: _Logs(future: _logs),
          ),
          JobChildren(future: _children, onOffset: _goToChildren),
        ],
      ),
    );
  }
}

class _Facts extends StatelessWidget {
  const _Facts({required this.job});

  final Job job;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        _row(context, Strings.columnStatus, child: JobStatusChip(job.status)),
        _row(
          context,
          Strings.columnPriority,
          value: jobPriorityLabel(job.priority),
        ),
        _row(
          context,
          Strings.columnProgress,
          value: '${(job.progress * 100).round()}%',
        ),
        _row(context, Strings.columnAttempts, value: '${job.attempts}'),
        _row(
          context,
          Strings.columnCreated,
          child: TimestampText(job.createdAt, relative: true),
        ),
        _row(
          context,
          Strings.columnUpdated,
          child: TimestampText(job.updatedAt, relative: true),
        ),
        if (job.startedAt != null)
          _row(
            context,
            Strings.columnStarted,
            child: TimestampText(job.startedAt, relative: true),
          ),
        if (job.finishedAt != null)
          _row(
            context,
            Strings.columnFinished,
            child: TimestampText(job.finishedAt, relative: true),
          ),
        if (job.parentId != null)
          _row(
            context,
            Strings.jobFactParent,
            child: _ParentLink(id: job.parentId!),
          ),
        if (job.lastError != null)
          _row(
            context,
            Strings.jobFactError,
            value: job.lastError!,
            danger: true,
          ),
      ],
    );
  }

  Widget _row(
    BuildContext context,
    String label, {
    String? value,
    Widget? child,
    bool danger = false,
  }) {
    final Tokens t = context.tokens;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: Space.s1),
      child: LayoutBuilder(
        builder: (BuildContext context, BoxConstraints constraints) => Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            SizedBox(
              width: constraints.maxWidth < Breakpoints.sm ? 92 : 120,
              child: Text(
                label,
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: t.muted),
              ),
            ),
            Expanded(
              child:
                  child ??
                  Text(
                    value ?? '',
                    style: TextStyle(color: danger ? t.danger : t.text),
                  ),
            ),
          ],
        ),
      ),
    );
  }
}

class _Logs extends StatelessWidget {
  const _Logs({required this.future});

  final Future<List<String>> future;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return FutureBuilder<List<String>>(
      future: future,
      builder: (BuildContext context, AsyncSnapshot<List<String>> snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const StatusText(Strings.loadingLogs);
        }
        if (snapshot.hasError) {
          return const StatusText(Strings.couldNotLoadLogs);
        }
        final List<String> lines = snapshot.data ?? const <String>[];
        if (lines.isEmpty) {
          return const StatusText(Strings.emptyLogs);
        }
        return Container(
          width: double.infinity,
          constraints: const BoxConstraints(maxHeight: 360),
          padding: const EdgeInsets.all(Space.s3),
          decoration: BoxDecoration(
            color: t.bg,
            borderRadius: const BorderRadius.all(Radii.sm),
            border: Border.all(color: t.border),
          ),
          child: SingleChildScrollView(
            child: SelectableText(
              lines.join('\n'),
              style: monoStyle.copyWith(color: t.text, fontSize: 12),
            ),
          ),
        );
      },
    );
  }
}

class _ParentLink extends StatelessWidget {
  const _ParentLink({required this.id});

  final String id;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Align(
      alignment: Alignment.centerLeft,
      child: InkWell(
        onTap: () => Navigator.of(context).pushReplacementNamed(jobRoute(id)),
        borderRadius: const BorderRadius.all(Radii.sm),
        child: Text(
          id,
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
