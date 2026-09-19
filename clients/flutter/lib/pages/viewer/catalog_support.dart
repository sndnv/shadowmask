import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user_library/item_state.dart';
import 'package:shadowmask/model/user_library/watched_rollup.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/empty_state.dart';

const int _batchLimit = 200;

String _key(TitleRef r) => r.key;

bool _rollsUp(TitleKind k) => k == TitleKind.series || k == TitleKind.season;

Future<List<ItemState>> _leafStates(
  CatalogApi catalog,
  String userId,
  List<TitleRef> leaves,
) async {
  if (leaves.isEmpty) {
    return const <ItemState>[];
  }
  return catalog.stateBatch(userId, leaves);
}

Future<Set<String>> _watchedRollups(
  CatalogApi catalog,
  String userId,
  List<TitleRef> targets,
) async {
  if (targets.isEmpty) {
    return const <String>{};
  }
  final List<WatchedRollup> rollups = await catalog.stateRollup(
    userId,
    targets,
  );
  return <String>{
    for (final WatchedRollup r in rollups)
      if (r.watched) _key(r.target),
  };
}

Future<void> tagWatched(
  CatalogApi catalog,
  String userId,
  List<CatalogCard> cards, {
  bool withProgress = false,
}) async {
  final List<TitleRef> leaves = cards
      .where((CatalogCard c) => c.ref.type.isLeaf)
      .map((CatalogCard c) => c.ref)
      .take(_batchLimit)
      .toList();
  final List<TitleRef> targets = cards
      .where((CatalogCard c) => _rollsUp(c.ref.type))
      .map((CatalogCard c) => c.ref)
      .take(_batchLimit)
      .toList();
  if (leaves.isEmpty && targets.isEmpty) {
    return;
  }
  final Set<String> watched = <String>{};
  final Map<String, int> percent = <String, int>{};
  final Map<String, ItemState> known = <String, ItemState>{};
  bool leavesAnswered = false;
  bool rollupsAnswered = false;
  await Future.wait<void>(<Future<void>>[
    _leafStates(catalog, userId, leaves)
        .then((List<ItemState> states) {
          leavesAnswered = true;
          for (final ItemState s in states) {
            known[_key(s.title)] = s;
            if (s.watched) {
              watched.add(_key(s.title));
            }
            if (s.progressPercent > 0) {
              percent[_key(s.title)] = s.progressPercent;
            }
          }
        })
        .catchError((Object _) {}),
    _watchedRollups(catalog, userId, targets)
        .then((Set<String> rolled) {
          rollupsAnswered = true;
          watched.addAll(rolled);
        })
        .catchError((Object _) {}),
  ]);
  for (final CatalogCard c in cards) {
    if (watched.contains(_key(c.ref))) {
      c.watched = true;
    }
    if (withProgress) {
      final int? found = percent[_key(c.ref)];
      if (found != null) {
        c.progressPercent = found;
      }
    }
    final ItemState? state = known[_key(c.ref)];
    if (state != null) {
      c.watchlisted = state.watchlisted;
      c.favorite = state.favorite;
      c.stateKnown = true;
    } else if (c.ref.type.isLeaf ? leavesAnswered : rollupsAnswered) {
      c.stateKnown = true;
    }
  }
}

Future<EmptyState> emptyState(
  ApiClient api, {
  required bool isAdmin,
  required String noun,
}) async {
  try {
    final List<Map<String, dynamic>> libraries = await api.getJsonArray(
      '/api/v1/libraries',
      (Map<String, dynamic> j) => j,
    );
    if (libraries.isEmpty) {
      return isAdmin
          ? EmptyState(
              Strings.emptyLibraries,
              actionLabel: Strings.createLibrary,
              actionRoute: adminLibrariesRoute(),
            )
          : const EmptyState(Strings.noLibrariesShared);
    }
  } catch (_) {}
  return EmptyState(noun);
}
