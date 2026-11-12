import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/app_version.dart';

class AppVersionFooter extends StatefulWidget {
  const AppVersionFooter({super.key, this.loadVersion = defaultVersion});

  final VersionLoader loadVersion;

  @override
  State<AppVersionFooter> createState() => _AppVersionFooterState();
}

class _AppVersionFooterState extends State<AppVersionFooter> {
  String? _version;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final String version = await widget.loadVersion();
      if (mounted) {
        setState(() => _version = version);
      }
    } on Exception {
      return;
    }
  }

  @override
  Widget build(BuildContext context) {
    final String? version = _version;
    return Padding(
      padding: const EdgeInsets.only(top: Space.s3, bottom: Space.s5),
      child: Text(
        version == null ? Strings.appTitle : '${Strings.appTitle} $version',
        textAlign: TextAlign.center,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: context.tokens.muted.withValues(alpha: 0.7),
        ),
      ),
    );
  }
}
