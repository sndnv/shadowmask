import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/l10n/strings.dart';

Future<bool> confirmSignOut(BuildContext context) => confirmDialog(
  context,
  title: Strings.signOut,
  message: Strings.confirmSignOut,
  confirmLabel: Strings.signOut,
  danger: false,
);

Future<bool> confirmSignOutEverywhere(BuildContext context) => confirmDialog(
  context,
  title: Strings.signOutEverywhere,
  message: Strings.confirmSignOutEverywhere,
  confirmLabel: Strings.signOutEverywhere,
);
