import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/admin/job_status_chip.dart';
import 'package:shadowmask/components/pagination.dart';
import 'package:shadowmask/components/progress_meter.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/timestamp_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/job/job_node.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/job_labels.dart';
import 'package:shadowmask/view/page.dart';

class JobChildren extends StatelessWidget {
  const JobChildren({super.key, required this.future, required this.onOffset});

  final Future<Paged<JobNode>> future;
  final void Function(int offset) onOffset;

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<Paged<JobNode>>(
      future: future,
      builder: (BuildContext context, AsyncSnapshot<Paged<JobNode>> snapshot) {
        if (snapshot.hasError) {
          return const SectionBlock(
            title: Strings.jobChildren,
            child: StatusText(Strings.couldNotLoadJobChildren),
          );
        }
        final Paged<JobNode>? page = snapshot.data;
        if (page == null || page.items.isEmpty) {
          return const SizedBox.shrink();
        }
        return SectionBlock(
          title: Strings.jobChildren,
          framed: false,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              _table(context, page.items),
              Pagination(
                basePath: '',
                params: const <String, String?>{},
                total: page.total,
                offset: page.offset,
                limit: page.limit,
                count: page.items.length,
                onOffset: onOffset,
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _table(BuildContext context, List<JobNode> nodes) =>
      AdminTable<JobNode>(
        rows: nodes,
        emptyText: Strings.emptyJobChildren,
        minWidth: 900,
        onRowTap: (JobNode n) =>
            Navigator.of(context).pushNamed(jobRoute(n.job.id)),
        rowLabel: Strings.openJob,
        rowColor: (JobNode n) => jobRowTint(context, n.job.status),
        columns: <AdminColumn<JobNode>>[
          AdminColumn<JobNode>(
            label: Strings.columnJob,
            size: AdminColumnSize.small,
            cell: (BuildContext c, JobNode n) => _IdCell(node: n),
          ),
          AdminColumn<JobNode>(
            label: Strings.columnKind,
            size: AdminColumnSize.large,
            essential: true,
            cell: (BuildContext c, JobNode n) =>
                Text(jobKindLabel(n.job.kind), overflow: TextOverflow.ellipsis),
          ),
          AdminColumn<JobNode>(
            label: Strings.columnStatus,
            essential: true,
            cell: (BuildContext c, JobNode n) => JobStatusChip(n.job.status),
          ),
          AdminColumn<JobNode>(
            label: Strings.columnProgress,
            cell: (BuildContext c, JobNode n) => ProgressMeter(
              percent: (n.job.progress * 100).round(),
              width: 70,
            ),
          ),
          AdminColumn<JobNode>(
            label: Strings.columnStarted,
            cell: (BuildContext c, JobNode n) => TimestampText(n.job.startedAt),
          ),
          AdminColumn<JobNode>(
            label: Strings.columnFinished,
            cell: (BuildContext c, JobNode n) =>
                TimestampText(n.job.finishedAt),
          ),
        ],
      );
}

class _IdCell extends StatelessWidget {
  const _IdCell({required this.node});

  final JobNode node;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Padding(
      padding: EdgeInsets.only(left: node.depth * Space.s4),
      child: Row(
        children: <Widget>[
          if (node.depth > 0) ...<Widget>[
            Icon(Icons.subdirectory_arrow_right, size: 14, color: t.muted),
            const SizedBox(width: Space.s1),
          ],
          Flexible(
            child: Text(
              shortJobId(node.job.id),
              overflow: TextOverflow.ellipsis,
              style: monoStyle.copyWith(color: t.text, fontSize: 12),
            ),
          ),
        ],
      ),
    );
  }
}
