import 'dart:math';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playlist_api.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/card_menu.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/playlist_dialogs.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/setting_help.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/title_heading.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/playlist/playlist.dart';
import 'package:shadowmask/model/playlist/playlist_detail.dart';
import 'package:shadowmask/model/playlist/playlist_item.dart';
import 'package:shadowmask/model/playlist/queue_entry.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/open_section.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/failure_reason.dart';
import 'package:shadowmask/view/play_route.dart';
import 'package:shadowmask/view/queue_next.dart';
import 'package:shadowmask/view/title_link.dart';

const double kPlaylistThumbWidth = 40;

class PlaylistsPage extends StatelessWidget {
  const PlaylistsPage({super.key, required this.api, this.id});

  final ApiClient api;
  final String? id;

  @override
  Widget build(BuildContext context) => SectionPage(
    api: api,
    section: NavSection.playlists,
    errorText: id == null
        ? Strings.couldNotLoadPlaylists
        : Strings.couldNotLoadPlaylist,
    loading: const SkeletonPage(child: SkeletonRows(rows: 4)),
    bodyBuilder: (BuildContext context, SelfUser user) => id == null
        ? _PlaylistsBody(api: api, user: user)
        : _PlaylistBody(api: api, id: id!, user: user),
  );
}

Widget _muted(BuildContext context, String text) =>
    Text(text, style: TextStyle(color: context.tokens.muted));

class _PlaylistsBody extends StatefulWidget {
  const _PlaylistsBody({required this.api, required this.user});

  final ApiClient api;
  final SelfUser user;

  @override
  State<_PlaylistsBody> createState() => _PlaylistsBodyState();
}

class _PlaylistsBodyState extends State<_PlaylistsBody>
    with RouteAware, Mutations<_PlaylistsBody> {
  late final PlaylistApi _playlists = PlaylistApi(widget.api);
  late Future<List<QueueEntry>> _queue = _loadQueue();
  late Future<List<Playlist>> _lists = _loadLists();
  List<QueueEntry> _entries = const <QueueEntry>[];
  int _queueRead = 0;
  int _listsRead = 0;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final ModalRoute<dynamic>? route = ModalRoute.of(context);
    if (route is PageRoute<dynamic>) {
      appRouteObserver.subscribe(this, route);
    }
  }

  @override
  void didPopNext() {
    if (mounted) {
      _reloadQueue();
      _reloadLists();
    }
  }

  @override
  void dispose() {
    appRouteObserver.unsubscribe(this);
    super.dispose();
  }

  Future<List<QueueEntry>> _loadQueue() async {
    final int read = ++_queueRead;
    final List<QueueEntry> entries = await _playlists.queue(widget.user.id);
    if (mounted && read == _queueRead) {
      setState(() => _entries = entries);
    }
    return entries;
  }

  Future<List<Playlist>> _loadLists() {
    _listsRead++;
    return _playlists.playlists(widget.user.id);
  }

  void _reloadQueue() => setState(() {
    _queue = _loadQueue()..ignore();
  });

  void _reloadLists() => setState(() {
    _lists = _loadLists()..ignore();
  });

  Future<void> _refreshQueue() async {
    final int read = ++_queueRead;
    final List<QueueEntry> entries;
    try {
      entries = await _playlists.queue(widget.user.id);
    } catch (_) {
      if (mounted && read == _queueRead) {
        _reloadQueue();
      }
      return;
    }
    if (mounted && read == _queueRead) {
      setState(() {
        _entries = entries;
        _queue = SynchronousFuture<List<QueueEntry>>(entries);
      });
    }
  }

  Future<void> _refreshLists() async {
    final int read = ++_listsRead;
    final List<Playlist> lists;
    try {
      lists = await _playlists.playlists(widget.user.id);
    } catch (_) {
      if (mounted && read == _listsRead) {
        _reloadLists();
      }
      return;
    }
    if (mounted && read == _listsRead) {
      setState(() {
        _lists = SynchronousFuture<List<Playlist>>(lists);
      });
    }
  }

  void _refreshAll() {
    _refreshQueue();
    _refreshLists();
  }

  void _play() {
    final QueueEntry? first = _entries.firstOrNull;
    if (first != null) {
      Navigator.of(context).pushNamed(first.card.route);
    }
  }

  Future<void> _clear() async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.clearQueueTitle,
      message: Strings.confirmClearQueue,
      confirmLabel: Strings.clearQueue,
    );
    if (!ok || !mounted) {
      return;
    }
    await mutate(
      () async {
        await _playlists.clearQueue(widget.user.id);
        await _refreshQueue();
      },
      errorText: Strings.errorAction,
      successText: Strings.toastQueueCleared,
    );
  }

  Future<void> _dequeue(CatalogCard card) async {
    final String? entryId = card.queueEntryId;
    if (entryId == null) {
      return;
    }
    await mutate(
      key: entryId,
      () async {
        await _playlists.dequeue(widget.user.id, entryId);
        await _refreshQueue();
      },
      errorText: Strings.errorRemove,
      successText: Strings.toastRemovedFromQueue(card.title),
      emphasis: card.title,
    );
  }

  Future<void> _enqueue(Playlist playlist, {required bool shuffle}) =>
      runBusy(() async {
        if (await _queuePlaylist(
              context,
              _playlists,
              widget.user.id,
              playlist,
              shuffle: shuffle,
            ) &&
            mounted) {
          await _refreshQueue();
        }
      });

  Future<void> _shuffle(Playlist playlist) => runBusy(() async {
    try {
      final PlaylistDetail detail = await _playlists.playlist(
        widget.user.id,
        playlist.id,
      );
      if (mounted) {
        await _playShuffled(context, widget.api, widget.user.id, detail.items);
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorAction, e));
      }
    }
  });

  Future<void> _rename(Playlist playlist) async {
    final String? name = await _askName(context, playlist);
    if (name == null || !mounted) {
      return;
    }
    await mutate(() async {
      await _playlists.rename(widget.user.id, playlist.id, name);
      await _refreshLists();
    }, errorText: Strings.errorAction);
  }

  Future<void> _delete(Playlist playlist) async {
    if (!await _askDelete(context, playlist) || !mounted) {
      return;
    }
    await mutate(
      () async {
        await _playlists.delete(widget.user.id, playlist.id);
        await _refreshLists();
      },
      errorText: Strings.errorAction,
      successText: Strings.toastPlaylistDeleted(playlist.name),
      emphasis: playlist.name,
    );
  }

  Future<void> _openMenu(Playlist playlist, Offset at) async {
    final bool idle = !busy();
    final bool filled = playlist.itemCount > 0;
    final _PlaylistMenu? chosen = await showMenu<_PlaylistMenu>(
      context: context,
      position: menuPositionAt(at, MediaQuery.sizeOf(context)),
      constraints: const BoxConstraints.tightFor(width: kCardMenuWidth),
      popUpAnimationStyle: AnimationStyle.noAnimation,
      items: <PopupMenuEntry<_PlaylistMenu>>[
        for (final _PlaylistMenu item in _PlaylistMenu.values)
          PopupMenuItem<_PlaylistMenu>(
            value: item,
            enabled: idle && (filled || !item.needsTitles),
            padding: const EdgeInsets.symmetric(horizontal: Space.s3),
            child: MenuRow(
              icon: item.icon,
              label: item.label,
              danger: item == _PlaylistMenu.delete,
            ),
          ),
      ],
    );
    if (chosen == null || !mounted) {
      return;
    }
    switch (chosen) {
      case _PlaylistMenu.queue:
        await _enqueue(playlist, shuffle: false);
      case _PlaylistMenu.shuffleQueue:
        await _enqueue(playlist, shuffle: true);
      case _PlaylistMenu.shuffle:
        await _shuffle(playlist);
      case _PlaylistMenu.rename:
        await _rename(playlist);
      case _PlaylistMenu.delete:
        await _delete(playlist);
    }
  }

  Future<void> _create() async {
    final String? name = await showPlaylistNameDialog(
      context,
      title: Strings.newPlaylist,
      submitLabel: Strings.create,
    );
    if (name == null || !mounted) {
      return;
    }
    await mutate(() async {
      await _playlists.create(widget.user.id, name);
      await _refreshLists();
    }, errorText: Strings.errorAction);
  }

  @override
  Widget build(BuildContext context) {
    final bool empty = _entries.isEmpty;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Breadcrumbs(const <Crumb>[Crumb(Strings.navigationPlaylists)]),
        SectionBlock(
          title: Strings.countedHeading(Strings.queueHeading, _entries.length),
          framed: false,
          actionItems: <PageAction>[
            PageAction(
              icon: Icons.play_arrow,
              label: Strings.playQueue,
              primary: true,
              onPressed: empty || busy() ? null : _play,
            ),
            PageAction(
              icon: Icons.clear_all,
              label: Strings.clearQueue,
              danger: true,
              onPressed: empty || busy() ? null : _clear,
            ),
          ],
          child: buildBlock<List<QueueEntry>>(
            future: _queue,
            errorText: Strings.couldNotLoadQueue,
            onRetry: _reloadQueue,
            loading: const SkeletonRows(rows: 2),
            builder: (BuildContext context, List<QueueEntry> entries) =>
                entries.isEmpty
                ? _muted(context, Strings.queueEmpty)
                : CardRail(
                    cards: <CatalogCard>[
                      for (final QueueEntry e in entries) e.card,
                    ],
                    imageBase: _playlists.imageBase,
                    onDismiss: _dequeue,
                    dismissesByTitle: true,
                    dismissTooltip: Strings.removeFromQueue,
                    dismissBusy: (CatalogCard c) => busy(c.queueEntryId),
                    onMenuAction: (CatalogCard _, CardAction _) =>
                        _refreshAll(),
                  ),
          ),
        ),
        const SizedBox(height: Space.s5),
        SectionBlock(
          title: Strings.playlistsHeading,
          actionItems: <PageAction>[
            PageAction(
              icon: Icons.add,
              label: Strings.newPlaylist,
              onPressed: busy() ? null : _create,
            ),
          ],
          child: buildBlock<List<Playlist>>(
            future: _lists,
            errorText: Strings.couldNotLoadPlaylists,
            onRetry: _reloadLists,
            loading: const SkeletonRows(rows: 3),
            builder: (BuildContext context, List<Playlist> lists) =>
                lists.isEmpty
                ? _muted(context, Strings.playlistsEmpty)
                : Material(
                    type: MaterialType.transparency,
                    child: Column(
                      children: <Widget>[
                        for (final Playlist p in lists)
                          GestureDetector(
                            onSecondaryTapUp: (TapUpDetails d) =>
                                _openMenu(p, d.globalPosition),
                            onLongPressStart: (LongPressStartDetails d) =>
                                _openMenu(p, d.globalPosition),
                            child: ListTile(
                              contentPadding: kSettingRowPadding,
                              dense: true,
                              visualDensity: VisualDensity.compact,
                              leading: const Icon(Icons.playlist_play),
                              title: Text(p.name),
                              subtitle: Text(
                                Strings.playlistItemCount(p.itemCount),
                              ),
                              trailing: const Icon(Icons.chevron_right),
                              onTap: () => Navigator.of(
                                context,
                              ).pushNamed(playlistRoute(p.id)),
                            ),
                          ),
                      ],
                    ),
                  ),
          ),
        ),
      ],
    );
  }
}

enum _PlaylistMenu {
  queue(Icons.queue_play_next, Strings.addToQueue),
  shuffleQueue(Icons.shuffle_on_outlined, Strings.shuffleIntoQueue),
  shuffle(Icons.shuffle, Strings.shufflePlay),
  rename(Icons.edit_outlined, Strings.renamePlaylist),
  delete(Icons.delete_outline, Strings.deletePlaylist);

  const _PlaylistMenu(this.icon, this.label);

  final IconData icon;
  final String label;

  bool get needsTitles => this != rename && this != delete;
}

Future<String?> _askName(BuildContext context, Playlist playlist) =>
    showPlaylistNameDialog(
      context,
      title: Strings.renamePlaylist,
      submitLabel: Strings.save,
      initial: playlist.name,
    );

Future<bool> _askDelete(BuildContext context, Playlist playlist) =>
    confirmDialog(
      context,
      title: Strings.deletePlaylist,
      message: Strings.confirmDeletePlaylist(playlist.name),
    );

Future<bool> _queuePlaylist(
  BuildContext context,
  PlaylistApi playlists,
  String userId,
  Playlist playlist, {
  required bool shuffle,
}) async {
  try {
    final int added = await playlists.enqueue(
      userId,
      playlist.id,
      shuffle: shuffle,
    );
    if (context.mounted) {
      Toasts.of(context).success(Strings.toastAddedToQueue(added));
    }
    return true;
  } catch (e) {
    if (context.mounted) {
      Toasts.of(context).error(failureText(Strings.errorAction, e));
    }
    return false;
  }
}

Future<void> _playShuffled(
  BuildContext context,
  ApiClient api,
  String userId,
  List<PlaylistItem> items,
) async {
  if (items.isEmpty) {
    return;
  }
  final CatalogCard card = items[Random().nextInt(items.length)].card;
  final String route = await playRouteFor(
    CatalogApi(api),
    userId,
    TitleLink(kind: card.ref.type, id: card.ref.id, label: queueLabel(card)),
  );
  if (context.mounted) {
    await Navigator.of(context).pushNamed(route);
  }
}

class _PlaylistBody extends StatefulWidget {
  const _PlaylistBody({
    required this.api,
    required this.id,
    required this.user,
  });

  final ApiClient api;
  final String id;
  final SelfUser user;

  @override
  State<_PlaylistBody> createState() => _PlaylistBodyState();
}

class _PlaylistBodyState extends State<_PlaylistBody>
    with RouteAware, Mutations<_PlaylistBody> {
  late final PlaylistApi _playlists = PlaylistApi(widget.api);
  late Future<PlaylistDetail> _future = _load();
  Playlist? _playlist;
  List<PlaylistItem> _items = const <PlaylistItem>[];

  String get _userId => widget.user.id;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final ModalRoute<dynamic>? route = ModalRoute.of(context);
    if (route is PageRoute<dynamic>) {
      appRouteObserver.subscribe(this, route);
    }
  }

  @override
  void didPopNext() {
    if (mounted) {
      _refresh();
    }
  }

  @override
  void dispose() {
    appRouteObserver.unsubscribe(this);
    super.dispose();
  }

  Future<PlaylistDetail> _load() async {
    final PlaylistDetail detail = await _playlists.playlist(_userId, widget.id);
    if (mounted) {
      setState(() {
        _playlist = detail.playlist;
        _items = detail.items;
      });
    }
    return detail;
  }

  void _retry() => setState(() {
    _future = _load();
  });

  Future<void> _refresh() async {
    try {
      await _load();
    } catch (_) {}
  }

  Future<void> _enqueue({required bool shuffle}) => runBusy(() async {
    final Playlist? playlist = _playlist;
    if (playlist == null) {
      return;
    }
    if (await _queuePlaylist(
          context,
          _playlists,
          _userId,
          playlist,
          shuffle: shuffle,
        ) &&
        mounted) {
      openCrumb(context, playlistsRoute());
    }
  });

  Future<void> _shuffle() =>
      runBusy(() => _playShuffled(context, widget.api, _userId, _items));

  Future<void> _rename() async {
    final Playlist? playlist = _playlist;
    if (playlist == null) {
      return;
    }
    final String? name = await _askName(context, playlist);
    if (name == null || !mounted) {
      return;
    }
    await mutate(
      () => _playlists.rename(_userId, widget.id, name),
      errorText: Strings.errorAction,
      then: _refresh,
    );
  }

  Future<void> _delete() async {
    final Playlist? playlist = _playlist;
    if (playlist == null) {
      return;
    }
    if (!await _askDelete(context, playlist) || !mounted) {
      return;
    }
    await mutate(
      () => _playlists.delete(_userId, widget.id),
      errorText: Strings.errorAction,
      successText: Strings.toastPlaylistDeleted(playlist.name),
      emphasis: playlist.name,
      then: () => Navigator.of(context).pushReplacementNamed(playlistsRoute()),
    );
  }

  Future<void> _remove(PlaylistItem item) => mutate(
    key: item.card.ref.key,
    () async {
      await _playlists.remove(_userId, widget.id, item.card.ref.id);
      await _refresh();
    },
    errorText: Strings.errorRemove,
    successText: Strings.toastRemovedFromPlaylist(item.card.title),
    emphasis: item.card.title,
  );

  Future<void> _reorder(int from, int to) async {
    if (from == to || busy()) {
      return;
    }
    final PlaylistItem item = _items[from];
    final int position = _items[to].position;
    final List<PlaylistItem> next = List<PlaylistItem>.of(_items);
    next.insert(to, next.removeAt(from));
    setState(() => _items = next);
    await runBusy(() async {
      try {
        await _playlists.move(_userId, widget.id, item.card.ref.id, position);
      } catch (e) {
        if (mounted) {
          Toasts.of(context).error(failureText(Strings.errorAction, e));
        }
      }
      await _refresh();
    });
  }

  @override
  Widget build(BuildContext context) => buildBlock<PlaylistDetail>(
    future: _future,
    errorText: Strings.couldNotLoadPlaylist,
    onRetry: _retry,
    builder: (BuildContext context, PlaylistDetail loaded) {
      final String name = _playlist?.name ?? loaded.playlist.name;
      final bool empty = _items.isEmpty;
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Breadcrumbs(<Crumb>[
            Crumb(Strings.navigationPlaylists, route: playlistsRoute()),
            Crumb(name),
          ]),
          TitleHeading(title: name),
          const SizedBox(height: Space.s5),
          SectionBlock(
            title: Strings.countedHeading(
              Strings.playlistTitlesHeading,
              _items.length,
            ),
            actionItems: <PageAction>[
              PageAction(
                icon: _PlaylistMenu.queue.icon,
                label: _PlaylistMenu.queue.label,
                primary: true,
                onPressed: empty || busy()
                    ? null
                    : () => _enqueue(shuffle: false),
              ),
              PageAction(
                icon: _PlaylistMenu.shuffleQueue.icon,
                label: _PlaylistMenu.shuffleQueue.label,
                onPressed: empty || busy()
                    ? null
                    : () => _enqueue(shuffle: true),
              ),
              PageAction(
                icon: _PlaylistMenu.shuffle.icon,
                label: _PlaylistMenu.shuffle.label,
                onPressed: empty || busy() ? null : _shuffle,
              ),
              PageAction(
                icon: _PlaylistMenu.rename.icon,
                label: _PlaylistMenu.rename.label,
                onPressed: busy() ? null : _rename,
              ),
              PageAction(
                icon: _PlaylistMenu.delete.icon,
                label: _PlaylistMenu.delete.label,
                danger: true,
                onPressed: busy() ? null : _delete,
              ),
            ],
            child: empty
                ? _muted(context, Strings.playlistEmpty)
                : Material(
                    type: MaterialType.transparency,
                    child: ReorderableListView.builder(
                      shrinkWrap: true,
                      physics: const NeverScrollableScrollPhysics(),
                      buildDefaultDragHandles: false,
                      itemCount: _items.length,
                      onReorderItem: _reorder,
                      itemBuilder: (BuildContext context, int index) =>
                          _PlaylistRow(
                            key: ValueKey<String>(_items[index].card.ref.key),
                            index: index,
                            item: _items[index],
                            imageBase: _playlists.imageBase,
                            busy: busy(_items[index].card.ref.key),
                            settling: busy(),
                            onRemove: () => _remove(_items[index]),
                          ),
                    ),
                  ),
          ),
        ],
      );
    },
  );
}

class _PlaylistRow extends StatelessWidget {
  const _PlaylistRow({
    super.key,
    required this.index,
    required this.item,
    required this.imageBase,
    required this.busy,
    required this.settling,
    required this.onRemove,
  });

  final int index;
  final PlaylistItem item;
  final String imageBase;
  final bool busy;
  final bool settling;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final CatalogCard card = item.card;
    final String detail = <String?>[card.subtitle, card.caption]
        .whereType<String>()
        .where((String part) => part.trim().isNotEmpty)
        .join(' · ');
    return InkWell(
      onTap: () => Navigator.of(context).pushNamed(card.route),
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: Space.s2,
          vertical: Space.s1,
        ),
        child: Row(
          children: <Widget>[
            _OwnTaps(
              child: ReorderableDragStartListener(
                index: index,
                enabled: !settling,
                child: const Tooltip(
                  message: Strings.reorderHandle,
                  child: Padding(
                    padding: EdgeInsets.all(Space.s2),
                    child: Icon(Icons.drag_handle),
                  ),
                ),
              ),
            ),
            const SizedBox(width: Space.s2),
            SizedBox(
              width: kPlaylistThumbWidth,
              child: CardArt(
                artwork: card.artwork,
                aspect: CardAspect.poster,
                imageBase: imageBase,
                width: kPlaylistThumbWidth,
              ),
            ),
            const SizedBox(width: Space.s3),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    card.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  if (detail.isNotEmpty)
                    Text(
                      detail,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(color: context.tokens.muted),
                    ),
                ],
              ),
            ),
            _OwnTaps(
              child: IconButton(
                icon: const Icon(Icons.close),
                tooltip: Strings.removeFromPlaylist,
                onPressed: busy || settling ? null : onRemove,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _OwnTaps extends StatelessWidget {
  const _OwnTaps({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => GestureDetector(
    behavior: HitTestBehavior.opaque,
    excludeFromSemantics: true,
    onTap: () {},
    child: child,
  );
}
