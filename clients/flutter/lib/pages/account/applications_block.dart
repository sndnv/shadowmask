import 'package:flutter/material.dart';

import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/desktop_entry.dart';

typedef DesktopEntryLoader = DesktopEntry? Function();

class ApplicationsBlock extends StatefulWidget {
  const ApplicationsBlock({super.key, this.load = currentDesktopEntry});

  final DesktopEntryLoader load;

  @override
  State<ApplicationsBlock> createState() => _ApplicationsBlockState();
}

class _ApplicationsBlockState extends State<ApplicationsBlock> {
  DesktopEntry? _entry;
  bool? _installed;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _entry = widget.load();
    _check();
  }

  Future<void> _check() async {
    final DesktopEntry? entry = _entry;
    if (entry == null) {
      return;
    }
    try {
      final bool installed = await entry.installed();
      if (mounted) {
        setState(() => _installed = installed);
      }
    } on Exception {
      return;
    }
  }

  Future<void> _toggle(DesktopEntry entry, bool installed) async {
    setState(() => _busy = true);
    try {
      if (installed) {
        await entry.remove();
      } else {
        await entry.add();
      }
      if (!mounted) {
        return;
      }
      setState(() {
        _busy = false;
        _installed = !installed;
      });
      Toasts.of(context).success(
        installed
            ? Strings.removedFromApplicationsMenu
            : Strings.addedToApplicationsMenu,
      );
    } on Exception {
      if (!mounted) {
        return;
      }
      setState(() => _busy = false);
      Toasts.of(context).error(Strings.applicationsMenuFailed);
    }
  }

  @override
  Widget build(BuildContext context) {
    final DesktopEntry? entry = _entry;
    final bool? installed = _installed;
    if (entry == null || installed == null) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    return SectionBlock(
      title: Strings.applicationsMenuHeading,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            Strings.applicationsMenuHelp,
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: t.muted),
          ),
          const SizedBox(height: Space.s3),
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton(
              onPressed: _busy ? null : () => _toggle(entry, installed),
              child: Text(
                installed
                    ? Strings.removeFromApplicationsMenu
                    : Strings.addToApplicationsMenu,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
