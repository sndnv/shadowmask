import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/pages/account/link_codes_block.dart';
import 'package:shadowmask/pages/account/profile_block.dart';
import 'package:shadowmask/pages/admin/library_access_block.dart';
import 'package:shadowmask/pages/default/section_page.dart';

class UserPage extends StatefulWidget {
  const UserPage({super.key, required this.api, this.id});

  final ApiClient api;
  final String? id;

  @override
  State<UserPage> createState() => _UserPageState();
}

class _UserPageState extends State<UserPage> {
  int _generation = 0;

  @override
  Widget build(BuildContext context) {
    final ApiClient api = widget.api;
    final String? id = widget.id;
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      bodyBuilder: (BuildContext context, SelfUser user) {
        if (!user.isAdmin) {
          return const StatusText(Strings.notAuthorized);
        }
        if (id == null || id.isEmpty) {
          return const StatusText(Strings.couldNotLoadUsers);
        }
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(Strings.adminHeading, route: adminRoute()),
              Crumb(Strings.adminUsers, route: adminUsersRoute()),
              Crumb(id),
            ]),
            ProfileBlock(
              api: api,
              userId: id,
              showAdminActions: id != user.id,
              onAccessRevoked: () => setState(() => _generation++),
            ),
            LibraryAccessBlock(api: api, userId: id),
            LinkCodesBlock(
              key: ValueKey<String>('codes-$_generation'),
              api: api,
              userId: id,
            ),
          ],
        );
      },
    );
  }
}
