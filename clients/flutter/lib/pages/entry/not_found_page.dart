import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/empty_note.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/empty_state.dart';

class NotFoundPage extends StatelessWidget {
  const NotFoundPage({super.key, required this.api, this.path});

  final ApiClient api;
  final String? path;

  @override
  Widget build(BuildContext context) {
    final String wanted =
        path ?? Uri.parse(ModalRoute.of(context)?.settings.name ?? '/').path;
    return SectionPage(
      api: api,
      section: NavSection.home,
      bodyBuilder: (BuildContext context, SelfUser user) => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Breadcrumbs(const <Crumb>[Crumb(Strings.notFoundHeading)]),
          const SizedBox(height: Space.s5),
          EmptyNote(
            EmptyState(
              Strings.notFoundBody,
              actionLabel: Strings.navigationHome,
              actionRoute: homeRoute(),
            ),
          ),
          const SizedBox(height: Space.s3),
          Center(
            child: Text(
              wanted,
              textAlign: TextAlign.center,
              style: monoStyle.copyWith(
                color: context.tokens.muted,
                fontSize: 12,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
