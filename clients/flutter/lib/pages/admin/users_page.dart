import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/user_admin_api.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/danger_icon_button.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/admin/form_dialog.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/pagination.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/timestamp_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/account_profile.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/failure_reason.dart';
import 'package:shadowmask/view/page.dart';

const List<UserRole> _creatableRoles = <UserRole>[
  UserRole.admin,
  UserRole.user,
  UserRole.player,
];

class UsersPage extends StatelessWidget {
  const UsersPage({super.key, required this.api, this.offset = 0});

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
          ? _UsersBody(
              users: UserAdminApi(api),
              offset: offset,
              selfId: user.id,
            )
          : const StatusText(Strings.notAuthorized),
    );
  }
}

class _UsersBody extends StatefulWidget {
  const _UsersBody({
    required this.users,
    required this.offset,
    required this.selfId,
  });

  final UserAdminApi users;
  final int offset;
  final String selfId;

  @override
  State<_UsersBody> createState() => _UsersBodyState();
}

class _UsersBodyState extends State<_UsersBody> with Mutations<_UsersBody> {
  late Future<Paged<AccountProfile>> _future = widget.users.users(
    offset: widget.offset,
  );

  void _reload() {
    setState(() {
      _future = widget.users.users(offset: widget.offset);
    });
  }

  Future<void> _openCreate() async {
    final bool? saved = await showDialog<bool>(
      context: context,
      builder: (BuildContext ctx) => _UserFormDialog(users: widget.users),
    );
    if (saved ?? false) {
      _reload();
    }
  }

  Future<void> _delete(AccountProfile user) async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.deleteUser,
      message: Strings.confirmDeleteUser(user.username),
    );
    if (!ok) {
      return;
    }
    await mutate(
      key: user.id,
      () => widget.users.deleteUser(user.id),
      successText: Strings.toastUserDeleted,
      errorText: Strings.errorDelete,
      then: _reload,
    );
  }

  Future<void> _setActive(AccountProfile user, bool active) async {
    if (!active) {
      final bool ok = await confirmDialog(
        context,
        title: Strings.deactivateUser,
        message: Strings.confirmDeactivateUser(user.username),
        confirmLabel: Strings.deactivate,
      );
      if (!ok) {
        return;
      }
    }
    await mutate(
      key: user.id,
      () => widget.users.setActive(user.id, active),
      successText: active
          ? Strings.toastUserActivated
          : Strings.toastUserDeactivated,
      errorText: Strings.errorSave,
      then: _reload,
    );
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<Paged<AccountProfile>>(
      future: _future,
      errorText: Strings.couldNotLoadUsers,
      builder: (BuildContext context, Paged<AccountProfile> page) {
        final Tokens t = context.tokens;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(Strings.adminHeading, route: adminRoute()),
              Crumb(Strings.countLabel(Strings.adminUsers, page.total)),
            ]),
            PageActions(<PageAction>[
              PageAction(
                icon: Icons.refresh,
                label: Strings.refresh,
                onPressed: _reload,
              ),
              PageAction(
                icon: Icons.add,
                label: Strings.createUser,
                primary: true,
                onPressed: _openCreate,
              ),
            ]),
            const SizedBox(height: Space.s4),
            AdminTable<AccountProfile>(
              rows: page.items,
              emptyText: Strings.emptyUsers,
              minWidth: 820,
              initialSortColumn: 0,
              onRowTap: (AccountProfile u) =>
                  Navigator.of(context).pushNamed(adminUserRoute(u.id)),
              rowLabel: Strings.openUser,
              rowColor: (AccountProfile u) => switch (u.role) {
                UserRole.admin => t.rowDanger,
                UserRole.player => t.rowWarn,
                _ => null,
              },
              columns: <AdminColumn<AccountProfile>>[
                AdminColumn<AccountProfile>(
                  label: Strings.columnUsername,
                  size: AdminColumnSize.large,
                  essential: true,
                  sortKey: (AccountProfile u) => u.username,
                  cell: (BuildContext c, AccountProfile u) =>
                      Text(u.username, overflow: TextOverflow.ellipsis),
                ),
                AdminColumn<AccountProfile>(
                  label: Strings.columnRole,
                  size: AdminColumnSize.small,
                  essential: true,
                  sortKey: (AccountProfile u) => u.role.name,
                  cell: (BuildContext c, AccountProfile u) => Text(u.role.name),
                ),
                AdminColumn<AccountProfile>(
                  label: Strings.columnStreams,
                  size: AdminColumnSize.small,
                  align: AdminColumnAlign.end,
                  sortKey: (AccountProfile u) => u.concurrentStreamLimit ?? -1,
                  cell: (BuildContext c, AccountProfile u) =>
                      Text(u.concurrentStreamLimit?.toString() ?? '—'),
                ),
                AdminColumn<AccountProfile>(
                  label: Strings.columnStatus,
                  size: AdminColumnSize.small,
                  sortKey: (AccountProfile u) => u.active ? 1 : 0,
                  cell: (BuildContext c, AccountProfile u) => Text(
                    u.active ? Strings.statusActive : Strings.statusInactive,
                    style: TextStyle(color: u.active ? t.text : t.muted),
                  ),
                ),
                AdminColumn<AccountProfile>(
                  label: Strings.columnCreated,
                  sortKey: (AccountProfile u) => u.createdAt,
                  cell: (BuildContext c, AccountProfile u) =>
                      TimestampText(u.createdAt),
                ),
                AdminColumn<AccountProfile>(
                  label: Strings.columnActions,
                  fixedWidth: adminActionsWidth(2),
                  align: AdminColumnAlign.end,
                  cell: (BuildContext c, AccountProfile u) {
                    final bool self = u.id == widget.selfId;
                    return Row(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        IconButton(
                          icon: Icon(
                            u.active
                                ? Icons.person_off_outlined
                                : Icons.person_outline,
                            size: 18,
                          ),
                          color: t.muted,
                          visualDensity: VisualDensity.compact,
                          tooltip: self && u.active
                              ? Strings.cannotDeactivateSelf
                              : (u.active
                                    ? Strings.deactivateUser
                                    : Strings.activateUser),
                          onPressed: (self && u.active) || busy(u.id)
                              ? null
                              : () => _setActive(u, !u.active),
                        ),
                        DangerIconButton(
                          icon: Icons.delete_outline,
                          tooltip: self
                              ? Strings.cannotDeleteSelf
                              : Strings.deleteUser,
                          onPressed: self || busy(u.id)
                              ? null
                              : () => _delete(u),
                        ),
                      ],
                    );
                  },
                ),
              ],
            ),
            const SizedBox(height: Space.s4),
            Pagination(
              basePath: adminUsersRoute(),
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

class _UserFormDialog extends StatefulWidget {
  const _UserFormDialog({required this.users});

  final UserAdminApi users;

  @override
  State<_UserFormDialog> createState() => _UserFormDialogState();
}

class _UserFormDialogState extends State<_UserFormDialog> {
  final TextEditingController _username = TextEditingController();
  final TextEditingController _password = TextEditingController();
  final TextEditingController _confirm = TextEditingController();
  UserRole _role = UserRole.user;
  bool _submitting = false;
  String? _usernameError;
  String? _passwordError;
  String? _confirmError;

  @override
  void dispose() {
    _username.dispose();
    _password.dispose();
    _confirm.dispose();
    super.dispose();
  }

  bool _validate() {
    final String? username = _username.text.trim().isEmpty
        ? Strings.requiredUsername
        : null;
    final String? password = _password.text.isEmpty
        ? Strings.requiredPassword
        : null;
    final String? confirm = _confirm.text != _password.text
        ? Strings.passwordsDoNotMatch
        : null;
    setState(() {
      _usernameError = username;
      _passwordError = password;
      _confirmError = confirm;
    });
    return username == null && password == null && confirm == null;
  }

  Future<void> _submit() async {
    if (!_validate()) {
      return;
    }
    final Map<String, dynamic> body = <String, dynamic>{
      'username': _username.text.trim(),
      'password': _password.text,
      'role': _role.name,
    };
    setState(() => _submitting = true);
    try {
      await widget.users.createUser(body);
      if (mounted) {
        Toasts.of(context).success(Strings.toastUserCreated);
        Navigator.of(context).pop(true);
      }
    } catch (e) {
      if (!mounted) {
        return;
      }
      final bool taken = failureCode(e) == kUsernameTaken;
      setState(() {
        _submitting = false;
        _usernameError = taken ? Strings.reasonUsernameTaken : null;
      });
      if (!taken) {
        Toasts.of(context).error(failureText(Strings.errorCreate, e));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return FormDialog(
      title: Strings.createUser,
      submitLabel: Strings.create,
      submitting: _submitting,
      onSubmit: _submit,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        spacing: Space.s3,
        children: <Widget>[
          LabelledTextField(
            controller: _username,
            label: Strings.fieldUsername,
            error: _usernameError,
            onChanged: (_) => _clearError(() => _usernameError = null),
          ),
          LabelledTextField(
            controller: _password,
            label: Strings.fieldPassword,
            obscure: true,
            error: _passwordError,
            onChanged: (_) => _clearError(() {
              _passwordError = null;
              _confirmError = null;
            }),
          ),
          LabelledTextField(
            controller: _confirm,
            label: Strings.fieldRepeatPassword,
            obscure: true,
            error: _confirmError,
            onChanged: (_) => _clearError(() => _confirmError = null),
          ),
          LabelledDropdown<UserRole>(
            label: Strings.fieldRole,
            value: _role,
            items: <(UserRole, String)>[
              for (final UserRole r in _creatableRoles) (r, r.name),
            ],
            onChanged: (UserRole v) => setState(() => _role = v),
          ),
        ],
      ),
    );
  }

  void _clearError(VoidCallback clear) {
    if (_usernameError != null ||
        _passwordError != null ||
        _confirmError != null) {
      setState(clear);
    }
  }
}
