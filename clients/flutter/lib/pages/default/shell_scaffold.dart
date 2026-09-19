import 'dart:math' as math;

import 'package:flutter/foundation.dart' show ValueListenable;
import 'package:flutter/material.dart';

import 'package:shadowmask/api/account_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/components/bottom_chrome.dart';
import 'package:shadowmask/components/brand_mark.dart';
import 'package:shadowmask/components/hex_texture.dart';
import 'package:shadowmask/components/page_title.dart';
import 'package:shadowmask/components/shell_bottom_nav.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_destination.dart';
import 'package:shadowmask/nav/nav_destinations.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/default/immersive_scope.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/scoped_value.dart';

const double _kHeaderHeight = kShellHeaderHeight;

const double kNavItemHPad = 10;
const double kNavItemGap = Space.s1;
const double kNavOverflowWidth = 40;
const TextStyle kNavItemText = TextStyle(
  fontSize: 13.5,
  fontWeight: FontWeight.w600,
);

double navItemWidth(String label, TextStyle style) {
  final TextPainter painter = TextPainter(
    text: TextSpan(text: label, style: style),
    textDirection: TextDirection.ltr,
    maxLines: 1,
  )..layout();
  return painter.width + kNavItemHPad * 2 + kNavItemGap;
}

int fittingNavItems({
  required double available,
  required List<double> widths,
  double overflowWidth = kNavOverflowWidth,
}) {
  double total = 0;
  for (final double w in widths) {
    total += w;
  }
  if (total <= available) {
    return widths.length;
  }
  final double room = math.max(0, available - overflowWidth);
  double used = 0;
  int fit = 0;
  for (final double w in widths) {
    if (used + w > room) {
      break;
    }
    used += w;
    fit++;
  }
  return fit;
}

class ShellScaffold extends StatefulWidget {
  const ShellScaffold({
    super.key,
    required this.api,
    required this.current,
    required this.body,
    this.user,
    this.fullWidth = false,
    this.fitViewport = false,
    this.keepsBackdrop = false,
  });

  final ApiClient api;
  final NavSection current;
  final Widget body;
  final SelfUser? user;
  final bool fullWidth;
  final bool fitViewport;
  final bool keepsBackdrop;

  @override
  State<ShellScaffold> createState() => _ShellScaffoldState();
}

const EdgeInsets _kBodyPadding = EdgeInsets.fromLTRB(
  Space.s4,
  Space.s3,
  Space.s4,
  Space.s4,
);
const EdgeInsets _kGutter = EdgeInsets.symmetric(horizontal: Space.s3);
const double _kOffscreen = 200;

class _ShellScaffoldState extends State<ShellScaffold> {
  final ScopedValue<String?> _ownBackdrop = ScopedValue<String?>(null);
  final ScopedValue<bool> _immersive = ScopedValue<bool>(false);
  final ScopedValue<String?> _title = ScopedValue<String?>(null);
  final FocusNode _main = FocusNode(debugLabel: 'main', skipTraversal: true);
  ScopedValue<String?>? _sharedBackdrop;
  bool _skipShown = false;
  bool _keepsBackdrop = false;
  bool _sweptBackdrop = false;

  ScopedValue<String?> get _backdrop => _sharedBackdrop ?? _ownBackdrop;

  void _keepBackdrop() => _keepsBackdrop = true;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _sharedBackdrop = BackdropScope.maybeOf(context);
    if (_sweptBackdrop) {
      return;
    }
    _sweptBackdrop = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && !_keepsBackdrop && !widget.keepsBackdrop) {
        _sharedBackdrop?.publish(null, owner: this);
      }
    });
  }

  @override
  void dispose() {
    _ownBackdrop.dispose();
    _immersive.dispose();
    _title.dispose();
    _main.dispose();
    super.dispose();
  }

  Widget _skipLink() => Positioned(
    top: _skipShown ? Space.s3 : -_kOffscreen,
    left: Space.s3,
    child: FocusTraversalOrder(
      order: const NumericFocusOrder(0),
      child: Focus(
        canRequestFocus: false,
        skipTraversal: true,
        onFocusChange: (bool has) => setState(() => _skipShown = has),
        child: FilledButton(
          onPressed: _main.requestFocus,
          child: const Text(Strings.skipToContent),
        ),
      ),
    ),
  );

  String? get _sectionLabel {
    for (final NavDestination d in navDestinations) {
      if (d.section == widget.current) {
        return d.label;
      }
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final double pageWidth = widget.fullWidth
        ? double.infinity
        : Breakpoints.lg;
    return PageTitleScope(
      title: _title,
      child: ValueListenableBuilder<String?>(
        valueListenable: _title,
        builder: (BuildContext context, String? page, Widget? shell) {
          final String? label = page ?? _sectionLabel;
          return Title(
            color: t.accent,
            title: label == null
                ? Strings.appTitle
                : Strings.documentTitle(label),
            child: shell!,
          );
        },
        child: _shell(
          t,
          pageWidth,
          MediaQuery.sizeOf(context).width < Breakpoints.sm,
          MediaQuery.paddingOf(context),
        ),
      ),
    );
  }

  Widget _shell(Tokens t, double pageWidth, bool compact, EdgeInsets safe) {
    final double headerExtent = _kHeaderHeight + safe.top;
    return FocusTraversalGroup(
      policy: OrderedTraversalPolicy(),
      child: Scaffold(
        backgroundColor: t.bg,
        bottomNavigationBar: !compact
            ? null
            : ValueListenableBuilder<bool>(
                valueListenable: _immersive,
                builder: (BuildContext context, bool immersive, Widget? bar) =>
                    immersive ? const SizedBox.shrink() : bar!,
                child: BottomChrome(
                  height: kNavHeight,
                  child: ShellBottomNav(
                    api: widget.api,
                    current: widget.current,
                    user: widget.user,
                  ),
                ),
              ),
        body: Stack(
          children: <Widget>[
            Positioned.fill(
              child: ValueListenableBuilder<String?>(
                valueListenable: _backdrop,
                builder: (BuildContext context, String? url, Widget? _) =>
                    BackdropWash(url: url),
              ),
            ),
            const Positioned.fill(child: HexTexture()),
            Positioned.fill(
              child: LayoutBuilder(
                builder: (BuildContext context, BoxConstraints constraints) =>
                    ValueListenableBuilder<bool>(
                      valueListenable: _immersive,
                      builder:
                          (BuildContext context, bool immersive, Widget? _) {
                            final bool bare = immersive || compact;
                            final double maxWidth = immersive
                                ? double.infinity
                                : pageWidth;
                            final double bodyHeight = math.max(
                              0,
                              constraints.maxHeight - (bare ? 0 : headerExtent),
                            );
                            final EdgeInsets gutter = immersive
                                ? EdgeInsets.zero
                                : _kGutter.add(
                                        EdgeInsets.only(
                                          left: safe.left,
                                          right: safe.right,
                                        ),
                                      )
                                      as EdgeInsets;
                            final EdgeInsets bodyPadding = immersive
                                ? EdgeInsets.zero
                                : _kBodyPadding.add(
                                        EdgeInsets.only(
                                          top: bare ? safe.top : 0,
                                          bottom: compact ? 0 : safe.bottom,
                                        ),
                                      )
                                      as EdgeInsets;
                            final BoxConstraints bodyConstraints;
                            if (immersive) {
                              bodyConstraints = BoxConstraints.tightFor(
                                height: constraints.maxHeight,
                              );
                            } else if (widget.fitViewport) {
                              bodyConstraints = BoxConstraints.tightFor(
                                height: bodyHeight,
                              );
                            } else {
                              bodyConstraints = BoxConstraints(
                                minHeight: bodyHeight,
                              );
                            }
                            return CustomScrollView(
                              physics: immersive || widget.fitViewport
                                  ? const NeverScrollableScrollPhysics()
                                  : null,
                              slivers: <Widget>[
                                SliverPersistentHeader(
                                  pinned: true,
                                  delegate: _ShellHeader(
                                    api: widget.api,
                                    current: widget.current,
                                    user: widget.user,
                                    maxWidth: maxWidth,
                                    backdrop: _backdrop,
                                    viewportHeight: constraints.maxHeight,
                                    collapsed: bare,
                                    insets: safe,
                                    tokens: t,
                                  ),
                                ),
                                SliverToBoxAdapter(
                                  child: ConstrainedBox(
                                    constraints: bodyConstraints,
                                    child: _Centered(
                                      maxWidth: maxWidth,
                                      padding: gutter,
                                      child: Padding(
                                        padding: bodyPadding,
                                        child: BackdropScope(
                                          url: _backdrop,
                                          onKeep: _keepBackdrop,
                                          child: ImmersiveScope(
                                            immersive: _immersive,
                                            child: Focus(
                                              focusNode: _main,
                                              child: widget.body,
                                            ),
                                          ),
                                        ),
                                      ),
                                    ),
                                  ),
                                ),
                              ],
                            );
                          },
                    ),
              ),
            ),
            _skipLink(),
          ],
        ),
      ),
    );
  }
}

class _Centered extends StatelessWidget {
  const _Centered({
    required this.maxWidth,
    required this.child,
    this.padding = _kGutter,
  });

  final double maxWidth;
  final Widget child;
  final EdgeInsets padding;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: padding,
      child: Align(
        alignment: Alignment.topCenter,
        child: ConstrainedBox(
          constraints: BoxConstraints(maxWidth: maxWidth),
          child: SizedBox(width: double.infinity, child: child),
        ),
      ),
    );
  }
}

class _ShellHeader extends SliverPersistentHeaderDelegate {
  const _ShellHeader({
    required this.api,
    required this.current,
    required this.user,
    required this.maxWidth,
    required this.backdrop,
    required this.viewportHeight,
    required this.collapsed,
    required this.insets,
    required this.tokens,
  });

  final ApiClient api;
  final NavSection current;
  final SelfUser? user;
  final double maxWidth;
  final ValueListenable<String?> backdrop;
  final double viewportHeight;
  final bool collapsed;
  final EdgeInsets insets;
  final Tokens tokens;

  @override
  double get minExtent => collapsed ? 0 : _kHeaderHeight + insets.top;

  @override
  double get maxExtent => collapsed ? 0 : _kHeaderHeight + insets.top;

  @override
  Widget build(BuildContext context, double shrinkOffset, bool _) {
    final Tokens t = tokens;
    if (collapsed) {
      return const SizedBox.shrink();
    }
    return Stack(
      fit: StackFit.expand,
      children: <Widget>[
        ColoredBox(color: t.bg),
        ClipRect(
          child: OverflowBox(
            alignment: Alignment.topCenter,
            minHeight: viewportHeight,
            maxHeight: viewportHeight,
            child: ValueListenableBuilder<String?>(
              valueListenable: backdrop,
              builder: (BuildContext context, String? url, Widget? _) =>
                  BackdropWash(url: url),
            ),
          ),
        ),
        const ClipRect(child: HexTexture()),
        Padding(
          padding: EdgeInsets.only(top: Space.s3 + insets.top),
          child: _Centered(
            maxWidth: maxWidth,
            padding:
                _kGutter.add(
                      EdgeInsets.only(left: insets.left, right: insets.right),
                    )
                    as EdgeInsets,
            child: Container(
              clipBehavior: Clip.antiAlias,
              decoration: BoxDecoration(
                color: t.surface,
                borderRadius: const BorderRadius.all(Radii.lg),
              ),
              foregroundDecoration: BoxDecoration(
                borderRadius: const BorderRadius.all(Radii.lg),
                border: Border.all(color: t.border),
              ),
              child: _TopNav(api: api, current: current, user: user),
            ),
          ),
        ),
      ],
    );
  }

  @override
  bool shouldRebuild(_ShellHeader old) =>
      old.current != current ||
      old.user != user ||
      old.maxWidth != maxWidth ||
      old.backdrop != backdrop ||
      old.viewportHeight != viewportHeight ||
      old.collapsed != collapsed ||
      old.insets != insets ||
      old.tokens != tokens;
}

class _TopNav extends StatelessWidget {
  const _TopNav({required this.api, required this.current, required this.user});

  final ApiClient api;
  final NavSection current;
  final SelfUser? user;

  Future<void> _signOut(BuildContext context) async {
    await AccountApi(api).signOutThisDevice(user?.id);
    if (context.mounted) {
      Navigator.of(
        context,
      ).pushNamedAndRemoveUntil('/', (Route<dynamic> r) => false);
    }
  }

  void _go(BuildContext context, String route) =>
      Navigator.of(context).pushReplacementNamed(route);

  Widget _brand(BuildContext context) => _Brand(
    onTap: () => Navigator.of(
      context,
    ).pushNamedAndRemoveUntil('/home', (Route<dynamic> r) => false),
  );

  Widget _account(BuildContext context) => _AccountLink(
    username: user!.username,
    active: current == NavSection.account,
    onTap: () => _go(context, '/account'),
  );

  Widget _signOutButton(BuildContext context, Tokens t) => TextButton(
    onPressed: () => _signOut(context),
    style: TextButton.styleFrom(
      foregroundColor: t.muted,
      padding: const EdgeInsets.symmetric(
        horizontal: Space.s2,
        vertical: Space.s1,
      ),
      minimumSize: Size.zero,
      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      textStyle: kNavItemText,
    ),
    child: const Text(Strings.signOut),
  );

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final List<NavDestination> all = destinationsFor(user).toList();
    final TextStyle style =
        (Theme.of(context).textTheme.labelLarge ?? const TextStyle()).merge(
          kNavItemText,
        );
    final List<double> widths = <double>[
      for (final NavDestination d in all) navItemWidth(d.label, style),
    ];
    return SizedBox(
      height: kNavHeight,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: Space.s4),
        child: Row(
          children: <Widget>[
            _brand(context),
            const SizedBox(width: Space.s2),
            Expanded(
              child: LayoutBuilder(
                builder: (BuildContext context, BoxConstraints constraints) =>
                    _destinations(context, all, widths, constraints.maxWidth),
              ),
            ),
            const SizedBox(width: Space.s3),
            if (user != null) ...<Widget>[
              _account(context),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: Space.s2),
                child: Text(
                  '/',
                  style: kNavItemText.copyWith(
                    color: t.muted,
                    fontWeight: FontWeight.w400,
                  ),
                ),
              ),
            ],
            _signOutButton(context, t),
          ],
        ),
      ),
    );
  }

  Widget _destinations(
    BuildContext context,
    List<NavDestination> all,
    List<double> widths,
    double available,
  ) {
    final int fit = fittingNavItems(available: available, widths: widths);
    return ClipRect(
      child: Row(
        children: <Widget>[
          for (int i = 0; i < fit; i++)
            _NavItem(
              label: all[i].label,
              active: all[i].section == current,
              admin: all[i].adminOnly,
              onTap: () => _go(context, all[i].route),
            ),
          if (fit < all.length)
            _NavMenu(
              destinations: all.sublist(fit),
              current: current,
              onSelect: (NavDestination d) => _go(context, d.route),
            ),
        ],
      ),
    );
  }
}

class _NavMenu extends StatelessWidget {
  const _NavMenu({
    required this.destinations,
    required this.current,
    required this.onSelect,
  });

  final List<NavDestination> destinations;
  final NavSection current;
  final ValueChanged<NavDestination> onSelect;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return MenuAnchor(
      alignmentOffset: kMenuOffset,
      style: appMenuStyle(t, minWidth: 180),
      menuChildren: <Widget>[
        for (final NavDestination d in destinations)
          MenuItemButton(
            onPressed: () => onSelect(d),
            leadingIcon: Icon(
              d.section == current ? Icons.chevron_right : null,
              size: 16,
              color: t.accent,
            ),
            child: Text(
              d.label,
              style: TextStyle(
                color: d.adminOnly ? t.accent : t.text,
                fontWeight: d.section == current
                    ? FontWeight.w700
                    : FontWeight.w500,
              ),
            ),
          ),
      ],
      builder: (BuildContext context, MenuController controller, Widget? _) =>
          SizedBox(
            width: kNavOverflowWidth,
            child: IconButton(
              onPressed: () =>
                  controller.isOpen ? controller.close() : controller.open(),
              icon: const Icon(Icons.more_horiz, size: 20),
              tooltip: Strings.navigationMore,
              color: t.text,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints.tightFor(
                width: kNavOverflowWidth,
                height: 32,
              ),
            ),
          ),
    );
  }
}

class _Brand extends StatelessWidget {
  const _Brand({required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: const BorderRadius.all(Radii.sm),
      child: const Padding(
        padding: EdgeInsets.all(Space.s1),
        child: BrandMark(size: 24),
      ),
    );
  }
}

class _AccountLink extends StatelessWidget {
  const _AccountLink({
    required this.username,
    required this.active,
    required this.onTap,
  });

  final String username;
  final bool active;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return TextButton(
      onPressed: onTap,
      style: ButtonStyle(
        backgroundColor: WidgetStateProperty.resolveWith<Color>((
          Set<WidgetState> states,
        ) {
          if (active) {
            return t.accent;
          }
          if (states.contains(WidgetState.hovered)) {
            return t.surfaceAlt;
          }
          return Colors.transparent;
        }),
        foregroundColor: WidgetStatePropertyAll<Color>(
          active ? t.accentContrast : t.text,
        ),
        overlayColor: const WidgetStatePropertyAll<Color>(Colors.transparent),
        padding: const WidgetStatePropertyAll<EdgeInsetsGeometry>(
          EdgeInsets.symmetric(horizontal: Space.s2, vertical: 6),
        ),
        minimumSize: const WidgetStatePropertyAll<Size>(Size.zero),
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
        shape: const WidgetStatePropertyAll<OutlinedBorder>(
          RoundedRectangleBorder(borderRadius: BorderRadius.all(Radii.sm)),
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          const Icon(Icons.person_outline, size: 16),
          const SizedBox(width: Space.s1),
          Text(
            username,
            style: monoStyle.copyWith(
              fontWeight: FontWeight.w700,
              fontSize: 13,
            ),
          ),
        ],
      ),
    );
  }
}

class _NavItem extends StatelessWidget {
  const _NavItem({
    required this.label,
    required this.active,
    required this.admin,
    required this.onTap,
  });

  final String label;
  final bool active;
  final bool admin;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Padding(
      padding: const EdgeInsets.only(right: kNavItemGap),
      child: TextButton(
        onPressed: onTap,
        style: ButtonStyle(
          backgroundColor: WidgetStateProperty.resolveWith<Color>((
            Set<WidgetState> states,
          ) {
            if (active) {
              return t.accent;
            }
            if (states.contains(WidgetState.hovered)) {
              return t.surfaceAlt;
            }
            return Colors.transparent;
          }),
          foregroundColor: WidgetStateProperty.resolveWith<Color>((
            Set<WidgetState> states,
          ) {
            if (active) {
              return t.accentContrast;
            }
            if (admin) {
              return t.accent;
            }
            if (states.contains(WidgetState.hovered)) {
              return t.text;
            }
            return t.muted;
          }),
          overlayColor: const WidgetStatePropertyAll<Color>(Colors.transparent),
          elevation: const WidgetStatePropertyAll<double>(0),
          padding: const WidgetStatePropertyAll<EdgeInsetsGeometry>(
            EdgeInsets.symmetric(horizontal: kNavItemHPad, vertical: 6),
          ),
          minimumSize: const WidgetStatePropertyAll<Size>(Size.zero),
          tapTargetSize: MaterialTapTargetSize.shrinkWrap,
          shape: WidgetStatePropertyAll<OutlinedBorder>(
            RoundedRectangleBorder(
              borderRadius: const BorderRadius.all(Radii.sm),
              side: admin && !active
                  ? BorderSide(color: t.border)
                  : BorderSide.none,
            ),
          ),
          textStyle: const WidgetStatePropertyAll<TextStyle>(kNavItemText),
        ),
        child: Text(label),
      ),
    );
  }
}
