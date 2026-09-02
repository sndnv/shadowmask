import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/segmented_tabs.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/model/user_library/favorite.dart';
import 'package:shadowmask/model/user_library/watchlist_item.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/account/appearance_block.dart';
import 'package:shadowmask/pages/account/devices_block.dart';
import 'package:shadowmask/pages/account/history_block.dart';
import 'package:shadowmask/pages/account/library_list_block.dart';
import 'package:shadowmask/pages/account/link_codes_block.dart';
import 'package:shadowmask/pages/account/profile_block.dart';
import 'package:shadowmask/pages/account/session_block.dart';
import 'package:shadowmask/pages/account/tokens_block.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/space.dart';

class AccountPage extends StatelessWidget {
  const AccountPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.account,
      errorText: Strings.couldNotLoadAccount,
      bodyBuilder: (BuildContext context, SelfUser user) =>
          _AccountBody(api: api, user: user),
    );
  }
}

class _AccountBody extends StatefulWidget {
  const _AccountBody({required this.api, required this.user});

  final ApiClient api;
  final SelfUser user;

  @override
  State<_AccountBody> createState() => _AccountBodyState();
}

class _AccountBodyState extends State<_AccountBody> {
  int _tab = 0;

  Widget _panel(List<Widget> children) => Column(
    mainAxisSize: MainAxisSize.min,
    crossAxisAlignment: CrossAxisAlignment.start,
    children: children,
  );

  @override
  Widget build(BuildContext context) {
    final ApiClient api = widget.api;
    final SelfUser user = widget.user;
    final CatalogApi catalog = CatalogApi(api);
    final bool management = !user.isPlayer;

    final List<(String, Widget)> tabs = <(String, Widget)>[
      (
        Strings.accountTabLibrary,
        _panel(<Widget>[
          LibraryListBlock(
            catalog: catalog,
            title: Strings.accountWatchlistHeading,
            emptyMessage: Strings.emptyWatchlist,
            loadRefs: () async => (await catalog.watchlist(
              user.id,
            )).map((WatchlistItem w) => w.title).toList(),
            onRemove: (TitleRef ref) =>
                catalog.removeFromWatchlist(user.id, ref),
            removeLabel: Strings.removeWatchlist,
          ),
          LibraryListBlock(
            catalog: catalog,
            title: Strings.accountFavoritesHeading,
            emptyMessage: Strings.emptyFavorites,
            loadRefs: () async => (await catalog.favorites(
              user.id,
            )).map((Favorite f) => f.title).toList(),
            onRemove: (TitleRef ref) => catalog.removeFavorite(user.id, ref),
            removeLabel: Strings.removeFavorite,
          ),
          HistoryBlock(api: catalog, userId: user.id),
        ]),
      ),
      (
        Strings.accountTabProfile,
        _panel(<Widget>[
          ProfileBlock(
            api: api,
            userId: user.id,
            editable: management,
            showPasswordAction: management,
          ),
          const AppearanceBlock(),
          SessionBlock(
            api: api,
            userId: user.id,
            showSignOutEverywhere: management,
          ),
        ]),
      ),
      if (management)
        (
          Strings.accountTabDevices,
          _panel(<Widget>[
            DevicesBlock(api: api, userId: user.id),
            LinkCodesBlock(api: api, userId: user.id),
            TokensBlock(api: api, userId: user.id),
          ]),
        ),
    ];

    final int index = _tab.clamp(0, tabs.length - 1);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Breadcrumbs(const <Crumb>[Crumb(Strings.navigationAccount)]),
        SegmentedTabs<int>(
          current: index,
          tabs: <(int, String)>[
            for (int i = 0; i < tabs.length; i++) (i, tabs[i].$1),
          ],
          onChanged: (int i) => setState(() => _tab = i),
        ),
        const SizedBox(height: Space.s4),
        TabPanel(
          label: tabs[index].$1,
          child: IndexedStack(
            index: index,
            children: <Widget>[
              for (final (String _, Widget panel) in tabs) panel,
            ],
          ),
        ),
      ],
    );
  }
}
