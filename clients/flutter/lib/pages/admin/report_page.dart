import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/report_admin_api.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/diagnostics/report_detail.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';

const JsonEncoder _pretty = JsonEncoder.withIndent('  ');

String reportJson(ReportDetail report) => _pretty.convert(report.toJson());

class ReportPage extends StatelessWidget {
  const ReportPage({super.key, required this.api, this.id});

  final ApiClient api;
  final String? id;

  @override
  Widget build(BuildContext context) {
    final String? id = this.id;
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      bodyBuilder: (BuildContext context, SelfUser user) {
        if (!user.isAdmin) {
          return const StatusText(Strings.notAuthorized);
        }
        if (id == null || id.isEmpty) {
          return const StatusText(Strings.couldNotLoadReport);
        }
        return _ReportBody(reports: ReportAdminApi(api), id: id);
      },
    );
  }
}

class _ReportBody extends StatefulWidget {
  const _ReportBody({required this.reports, required this.id});

  final ReportAdminApi reports;
  final String id;

  @override
  State<_ReportBody> createState() => _ReportBodyState();
}

class _ReportBodyState extends State<_ReportBody> with Mutations<_ReportBody> {
  late final Future<ReportDetail> _future = widget.reports.report(widget.id);

  Future<void> _copy(ReportDetail report) async {
    await Clipboard.setData(ClipboardData(text: reportJson(report)));
    if (mounted) {
      Toasts.of(context).success(Strings.toastReportCopied);
    }
  }

  Future<void> _delete() async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.deleteReportTitle,
      message: Strings.deleteReportMessage,
    );
    if (!ok) {
      return;
    }
    await mutate(
      () => widget.reports.delete(widget.id),
      successText: Strings.toastReportDeleted,
      errorText: Strings.errorDeleteReport,
      then: () =>
          Navigator.of(context).pushReplacementNamed(adminReportsRoute()),
    );
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<ReportDetail>(
      future: _future,
      errorText: Strings.couldNotLoadReport,
      builder: (BuildContext context, ReportDetail report) {
        final Tokens t = context.tokens;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(Strings.adminHeading, route: adminRoute()),
              Crumb(Strings.adminReports, route: adminReportsRoute()),
              Crumb(
                preciseDateTimeText(report.summary.createdAt) ??
                    report.summary.id,
              ),
            ]),
            PageActions(<PageAction>[
              PageAction(
                icon: Icons.copy,
                label: Strings.copyReport,
                onPressed: () => _copy(report),
              ),
              PageAction(
                icon: Icons.delete_outline,
                label: Strings.delete,
                danger: true,
                onPressed: busy() ? null : _delete,
              ),
            ]),
            const SizedBox(height: Space.s4),
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(Space.s3),
              decoration: BoxDecoration(
                color: t.bg,
                borderRadius: const BorderRadius.all(Radii.sm),
                border: Border.all(color: t.border),
              ),
              child: SelectionArea(
                child: Text(
                  reportJson(report),
                  style: monoStyle.copyWith(
                    color: t.text,
                    fontSize: 12,
                    height: 1.4,
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}
