import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/user_admin_api.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/pagination.dart';
import 'package:shadowmask/components/progress_meter.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/timestamp_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/resume_card.dart';
import 'package:shadowmask/model/session/now_playing.dart';
import 'package:shadowmask/model/user/account_profile.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/view/page.dart';

class ActivityPage extends StatelessWidget {
  const ActivityPage({super.key, required this.api, this.offset = 0});

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
          ? _ActivityBody(
              admin: AdminApi(api),
              users: UserAdminApi(api),
              offset: offset,
            )
          : const StatusText(Strings.notAuthorized),
    );
  }
}

typedef _ActivityData = ({Paged<NowPlaying> page, Map<String, String> names});

class _ActivityBody extends StatefulWidget {
  const _ActivityBody({
    required this.admin,
    required this.users,
    required this.offset,
  });

  final AdminApi admin;
  final UserAdminApi users;
  final int offset;

  @override
  State<_ActivityBody> createState() => _ActivityBodyState();
}

class _ActivityBodyState extends State<_ActivityBody> {
  late Future<_ActivityData> _future = _load();

  Future<_ActivityData> _load() async {
    final Future<Map<String, String>> pendingNames = widget.users
        .users(limit: 200)
        .then(
          (Paged<AccountProfile> page) => <String, String>{
            for (final AccountProfile u in page.items) u.id: u.username,
          },
        )
        .catchError((Object _) => <String, String>{});
    final Paged<NowPlaying> page = await widget.admin.activity(
      offset: widget.offset,
    );
    return (page: page, names: await pendingNames);
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<_ActivityData>(
      future: _future,
      errorText: Strings.couldNotLoadActivity,
      builder: (BuildContext context, _ActivityData data) {
        final Paged<NowPlaying> page = data.page;
        final Tokens t = context.tokens;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(Strings.adminHeading, route: adminRoute()),
              Crumb(Strings.countLabel(Strings.adminActivity, page.total)),
            ]),
            PageActions(<PageAction>[
              PageAction(
                icon: Icons.refresh,
                label: Strings.refresh,
                onPressed: _reload,
              ),
            ]),
            const SizedBox(height: Space.s4),
            AdminTable<NowPlaying>(
              rows: page.items,
              emptyText: Strings.emptyActivity,
              minWidth: 960,
              initialSortColumn: 0,
              rowColor: (NowPlaying n) => switch (n.state) {
                'playing' => t.rowOk,
                'paused' => t.rowWarn,
                _ => null,
              },
              columns: <AdminColumn<NowPlaying>>[
                AdminColumn<NowPlaying>(
                  label: Strings.columnUsername,
                  essential: true,
                  sortKey: (NowPlaying n) => data.names[n.userId] ?? n.userId,
                  cell: (BuildContext c, NowPlaying n) => _RowLink(
                    label: data.names[n.userId] ?? n.userId,
                    route: adminUserRoute(n.userId),
                  ),
                ),
                AdminColumn<NowPlaying>(
                  label: Strings.columnTitle,
                  size: AdminColumnSize.large,
                  essential: true,
                  sortKey: (NowPlaying n) =>
                      n.card?.displayTitle ?? n.versionId,
                  cell: (BuildContext c, NowPlaying n) {
                    final ResumeCard? card = n.card;
                    if (card == null) {
                      return Text(
                        n.versionId,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      );
                    }
                    return _RowLink(
                      label: card.displayTitle,
                      route: titleRoute(card.title.type, card.title.id),
                    );
                  },
                ),
                AdminColumn<NowPlaying>(
                  label: Strings.columnProgress,
                  size: AdminColumnSize.small,
                  essential: true,
                  sortKey: (NowPlaying n) => n.card?.progressPercent ?? 0,
                  cell: (BuildContext c, NowPlaying n) =>
                      ProgressMeter(percent: n.card?.progressPercent ?? 0),
                ),
                AdminColumn<NowPlaying>(
                  label: Strings.columnStatus,
                  size: AdminColumnSize.small,
                  sortKey: (NowPlaying n) => n.state,
                  cell: (BuildContext c, NowPlaying n) => Text(n.state),
                ),
                AdminColumn<NowPlaying>(
                  label: Strings.factDuration,
                  size: AdminColumnSize.small,
                  align: AdminColumnAlign.end,
                  sortKey: (NowPlaying n) => n.positionMs,
                  cell: (BuildContext c, NowPlaying n) =>
                      Text(clock(n.positionMs)),
                ),
                AdminColumn<NowPlaying>(
                  label: Strings.columnStarted,
                  sortKey: (NowPlaying n) => n.startedAt,
                  cell: (BuildContext c, NowPlaying n) =>
                      TimestampText(n.startedAt),
                ),
                AdminColumn<NowPlaying>(
                  label: Strings.columnLastHeartbeat,
                  sortKey: (NowPlaying n) => n.lastHeartbeatAt,
                  cell: (BuildContext c, NowPlaying n) =>
                      TimestampText(n.lastHeartbeatAt),
                ),
              ],
            ),
            const SizedBox(height: Space.s4),
            Pagination(
              basePath: adminActivityRoute(),
              params: const <String, String?>{},
              total: page.total,
              offset: page.offset,
              limit: page.limit,
              count: page.items.length,
            ),
          ],
        );
      },
    );
  }
}

class _RowLink extends StatelessWidget {
  const _RowLink({required this.label, required this.route});

  final String label;
  final String route;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Align(
      alignment: Alignment.centerLeft,
      child: HoverTap(
        onTap: () => Navigator.of(context).pushNamed(route),
        child: Text(
          label,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(
            color: t.accent,
            decoration: TextDecoration.underline,
            decorationColor: t.accent,
          ),
        ),
      ),
    );
  }
}
