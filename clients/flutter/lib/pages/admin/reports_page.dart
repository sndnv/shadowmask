import 'dart:async';

import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/report_admin_api.dart';
import 'package:shadowmask/api/user_admin_api.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_filter_field.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/pagination.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/diagnostics/report_summary.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/view/page.dart';

class ReportsPage extends StatelessWidget {
  const ReportsPage({super.key, required this.api, this.offset = 0});

  final ApiClient api;
  final int offset;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      loading: const SkeletonPage(child: SkeletonRows()),
      bodyBuilder: (BuildContext context, SelfUser user) => user.isAdmin
          ? _ReportsBody(
              reports: ReportAdminApi(api),
              users: UserAdminApi(api),
              offset: offset,
            )
          : const StatusText(Strings.notAuthorized),
    );
  }
}

typedef _ReportsData = ({Paged<ReportSummary> page, Map<String, String> names});

class _ReportsBody extends StatefulWidget {
  const _ReportsBody({
    required this.reports,
    required this.users,
    required this.offset,
  });

  final ReportAdminApi reports;
  final UserAdminApi users;
  final int offset;

  @override
  State<_ReportsBody> createState() => _ReportsBodyState();
}

class _ReportsBodyState extends State<_ReportsBody> {
  late int _offset = widget.offset;
  late Future<_ReportsData> _future = _load();
  final TextEditingController _filter = TextEditingController();
  Timer? _debounce;
  String _needle = '';
  int? _total;

  @override
  void dispose() {
    _debounce?.cancel();
    _filter.dispose();
    super.dispose();
  }

  Future<_ReportsData> _load() {
    final Future<_ReportsData> pending = _fetch();
    pending.then((_ReportsData data) {
      if (!mounted ||
          !identical(_future, pending) ||
          data.page.total == _total) {
        return;
      }
      setState(() => _total = data.page.total);
    }, onError: (Object _, StackTrace _) {});
    return pending;
  }

  Future<_ReportsData> _fetch() async {
    final Future<Map<String, String>> names = widget.users.names().catchError(
      (Object _) => <String, String>{},
    );
    final Paged<ReportSummary> page = await widget.reports.reports(
      offset: _offset,
      filter: _needle,
    );
    return (page: page, names: await names);
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

  @override
  Widget build(BuildContext context) {
    final int? total = _total;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Breadcrumbs(<Crumb>[
          Crumb(Strings.adminHeading, route: adminRoute()),
          Crumb(
            total == null
                ? Strings.adminReports
                : Strings.countLabel(Strings.adminReports, total),
          ),
        ]),
        PageActions(<PageAction>[
          PageAction(
            icon: Icons.refresh,
            label: Strings.refresh,
            onPressed: _reload,
          ),
        ]),
        const SizedBox(height: Space.s4),
        AdminFilterField(
          controller: _filter,
          hintText: Strings.filterReports,
          onChanged: _onFilterChanged,
        ),
        const SizedBox(height: Space.s3),
        buildBlock<_ReportsData>(
          future: _future,
          errorText: Strings.couldNotLoadReports,
          builder: _table,
        ),
      ],
    );
  }

  Widget _table(BuildContext context, _ReportsData data) {
    final Paged<ReportSummary> page = data.page;
    String name(ReportSummary r) => data.names[r.userId] ?? r.userId;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        AdminTable<ReportSummary>(
          rows: page.items,
          emptyText: _needle.isEmpty
              ? Strings.emptyReports
              : Strings.noMatchingReports,
          minWidth: 960,
          initialSortColumn: 0,
          initialSortAscending: false,
          onRowTap: (ReportSummary r) =>
              Navigator.of(context).pushNamed(adminReportRoute(r.id)),
          columns: <AdminColumn<ReportSummary>>[
            AdminColumn<ReportSummary>(
              label: Strings.reportSent,
              essential: true,
              sortKey: (ReportSummary r) => r.createdAt,
              cell: (BuildContext c, ReportSummary r) =>
                  _cell(preciseDateTimeText(r.createdAt) ?? ''),
            ),
            AdminColumn<ReportSummary>(
              label: Strings.reportPlayer,
              essential: true,
              sortKey: (ReportSummary r) => r.playerId,
              cell: (BuildContext c, ReportSummary r) => _cell(r.playerId),
            ),
            AdminColumn<ReportSummary>(
              label: Strings.reportUser,
              sortKey: name,
              cell: (BuildContext c, ReportSummary r) => _cell(name(r)),
            ),
            AdminColumn<ReportSummary>(
              label: Strings.reportApp,
              size: AdminColumnSize.small,
              sortKey: (ReportSummary r) => r.app,
              cell: (BuildContext c, ReportSummary r) => _cell(r.app),
            ),
            AdminColumn<ReportSummary>(
              label: Strings.reportAppVersion,
              size: AdminColumnSize.small,
              sortKey: (ReportSummary r) => r.appVersion,
              cell: (BuildContext c, ReportSummary r) => _cell(r.appVersion),
            ),
            AdminColumn<ReportSummary>(
              label: Strings.reportPlatform,
              size: AdminColumnSize.large,
              essential: true,
              sortKey: (ReportSummary r) => r.platform,
              cell: (BuildContext c, ReportSummary r) => _cell(r.platform),
            ),
          ],
        ),
        const SizedBox(height: Space.s4),
        Pagination(
          basePath: adminReportsRoute(),
          params: const <String, String?>{},
          total: page.total,
          offset: page.offset,
          limit: page.limit,
          count: page.items.length,
          onOffset: _goTo,
        ),
      ],
    );
  }

  Widget _cell(String text) =>
      Text(text, maxLines: 1, overflow: TextOverflow.ellipsis);
}
