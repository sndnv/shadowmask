import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/server/rating_system.dart';
import 'package:shadowmask/model/server/server_info.dart';
import 'package:shadowmask/model/user/account_profile.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/pages/account/password_dialog.dart';
import 'package:shadowmask/pages/account/profile_edit_dialog.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class ProfileBlock extends StatefulWidget {
  const ProfileBlock({
    super.key,
    required this.api,
    required this.userId,
    this.editable = true,
    this.showPasswordAction = false,
  });

  final ApiClient api;
  final String userId;
  final bool editable;
  final bool showPasswordAction;

  @override
  State<ProfileBlock> createState() => _ProfileBlockState();
}

class _ProfileBlockState extends State<ProfileBlock> {
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late Future<AccountProfile> _future = _catalog.user(widget.userId);
  AccountProfile? _profile;

  Future<void> _edit() async {
    final AccountProfile? profile = _profile;
    if (profile == null) {
      return;
    }
    final List<RatingSystem> systems = await PlaybackApi(widget.api)
        .serverInfo()
        .then((ServerInfo info) => info.ratingSystems)
        .catchError((Object _) => const <RatingSystem>[]);
    if (!mounted) {
      return;
    }
    final bool saved = await ProfileEditDialog.show(
      context,
      catalog: _catalog,
      userId: widget.userId,
      profile: profile,
      ratingSystems: systems,
    );
    if (saved && mounted) {
      setState(() {
        _future = _catalog.user(widget.userId);
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return SectionBlock(
      title: Strings.accountProfileHeading,
      actionItems: <PageAction>[
        if (widget.editable)
          PageAction(
            icon: Icons.edit_outlined,
            label: Strings.editProfile,
            onPressed: _edit,
          ),
        if (widget.showPasswordAction)
          PageAction(
            icon: Icons.lock_outline,
            label: Strings.changePassword,
            onPressed: () => PasswordDialog.show(
              context,
              api: widget.api,
              userId: widget.userId,
            ),
          ),
      ],
      child: buildBlock<AccountProfile>(
        future: _future,
        errorText: Strings.couldNotLoadAccount,
        builder: (BuildContext context, AccountProfile p) {
          _profile = p;
          return _facts(context, p);
        },
      ),
    );
  }

  Widget _facts(BuildContext context, AccountProfile p) {
    final List<(String, String?)> facts = <(String, String?)>[
      (Strings.username, p.username),
      (Strings.fieldRole, p.role.name),
      (
        Strings.fieldPreferredAudio,
        p.preferredAudio.isEmpty ? null : p.preferredAudio.join(', '),
      ),
      (
        Strings.fieldPreferredSubtitle,
        p.preferredSubtitle.isEmpty ? null : p.preferredSubtitle.join(', '),
      ),
      (Strings.fieldMaximumRating, p.maxContentRating?.label),
      (Strings.fieldConcurrentStreams, p.concurrentStreamLimit?.toString()),
      (Strings.fieldBitrateCap, p.bitrateCap?.toString()),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: Space.s2,
      children: <Widget>[
        for (final (String, String?) f in facts)
          _FactLine(label: f.$1, value: f.$2),
      ],
    );
  }
}

class _FactLine extends StatelessWidget {
  const _FactLine({required this.label, required this.value});

  final String label;
  final String? value;

  @override
  Widget build(BuildContext context) {
    final bool unset = value == null;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        SizedBox(
          width: 160,
          child: Text(
            label,
            style: Theme.of(
              context,
            ).textTheme.bodyMedium?.copyWith(color: context.tokens.muted),
          ),
        ),
        Expanded(
          child: Text(
            value ?? Strings.notSet,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: unset ? context.tokens.muted : null,
              fontStyle: unset ? FontStyle.italic : null,
            ),
          ),
        ),
      ],
    );
  }
}
