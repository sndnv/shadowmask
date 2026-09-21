import 'package:flutter/material.dart';
import 'package:package_info_plus/package_info_plus.dart';

import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

typedef VersionLoader = Future<String> Function();

Future<String> defaultVersion() async =>
    (await PackageInfo.fromPlatform()).version;

class AboutBlock extends StatefulWidget {
  const AboutBlock({super.key, this.loadVersion = defaultVersion});

  final VersionLoader loadVersion;

  @override
  State<AboutBlock> createState() => _AboutBlockState();
}

class _AboutBlockState extends State<AboutBlock> {
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
    final Tokens t = context.tokens;
    final TextStyle? small = Theme.of(
      context,
    ).textTheme.bodySmall?.copyWith(color: t.muted);
    final String? version = _version;
    return SectionBlock(
      title: Strings.accountAboutHeading,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            version == null ? Strings.appTitle : '${Strings.appTitle} $version',
            style: const TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: Space.s2),
          Text(Strings.aboutLegalese, style: small),
          const SizedBox(height: Space.s2),
          Text(Strings.aboutBundledHelp, style: small),
          const SizedBox(height: Space.s3),
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton(
              onPressed: () => showLicensePage(
                context: context,
                applicationName: Strings.appTitle,
                applicationVersion: version,
                applicationLegalese: Strings.aboutLegalese,
              ),
              child: const Text(Strings.viewThirdPartyLicenses),
            ),
          ),
          const SizedBox(height: Space.s4),
          Text(
            Strings.aboutMetadataHeading,
            style: const TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: Space.s2),
          Image.asset(
            'assets/attribution/tmdb-logo.png',
            width: 120,
            semanticLabel: Strings.aboutTmdbLogoLabel,
          ),
          const SizedBox(height: Space.s2),
          Text(Strings.aboutTmdbNotice, style: small),
          const SizedBox(height: Space.s2),
          Text(Strings.aboutOmdbNotice, style: small),
          const SizedBox(height: Space.s2),
          Text(Strings.aboutSubtitleProvider, style: small),
        ],
      ),
    );
  }
}
