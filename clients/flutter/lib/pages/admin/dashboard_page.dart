import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/admin_link_card.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/space.dart';

class DashboardPage extends StatelessWidget {
  const DashboardPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      bodyBuilder: (BuildContext context, SelfUser user) => user.isAdmin
          ? _body(context)
          : const StatusText(Strings.notAuthorized),
    );
  }

  Widget _body(BuildContext context) {
    void go(String route) => Navigator.of(context).pushNamed(route);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Breadcrumbs(const <Crumb>[Crumb(Strings.adminHeading)]),
        const SizedBox(height: Space.s2),
        _Group(
          heading: Strings.adminGroupContent,
          children: <Widget>[
            AdminLinkCard(
              title: Strings.adminLibraries,
              description: Strings.adminLibrariesAbout,
              icon: Icons.video_library_outlined,
              onTap: () => go(adminLibrariesRoute()),
            ),
            AdminLinkCard(
              title: Strings.adminCollections,
              description: Strings.adminCollectionsAbout,
              icon: Icons.collections_bookmark_outlined,
              onTap: () => go(adminCollectionsRoute()),
            ),
            AdminLinkCard(
              title: Strings.adminVersions,
              description: Strings.adminVersionsAbout,
              icon: Icons.movie_outlined,
              onTap: () => go(adminVersionsRoute()),
            ),
            AdminLinkCard(
              title: Strings.adminFetch,
              description: Strings.adminFetchAbout,
              icon: Icons.cloud_download_outlined,
              onTap: () => go(adminFetchRoute()),
            ),
          ],
        ),
        const SizedBox(height: Space.s6),
        _Group(
          heading: Strings.adminGroupOperations,
          children: <Widget>[
            AdminLinkCard(
              title: Strings.adminUsers,
              description: Strings.adminUsersAbout,
              icon: Icons.people_outline,
              onTap: () => go(adminUsersRoute()),
            ),
            AdminLinkCard(
              title: Strings.adminJobs,
              description: Strings.adminJobsAbout,
              icon: Icons.work_outline,
              onTap: () => go(adminJobsRoute()),
            ),
            AdminLinkCard(
              title: Strings.adminActivity,
              description: Strings.adminActivityAbout,
              icon: Icons.bolt_outlined,
              onTap: () => go(adminActivityRoute()),
            ),
          ],
        ),
      ],
    );
  }
}

class _Group extends StatelessWidget {
  const _Group({required this.heading, required this.children});

  final String heading;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Text(heading, style: Theme.of(context).textTheme.headlineMedium),
        const SizedBox(height: Space.s3),
        Wrap(spacing: Space.s4, runSpacing: Space.s4, children: children),
      ],
    );
  }
}
