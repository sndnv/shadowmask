import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/widgets.dart';

import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/player/trickplay_cell.dart';
import 'package:shadowmask/components/player/trickplay_tile.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';

class TrickplayLoader {
  TrickplayLoader({
    required this.api,
    required this.versionId,
    required this.refs,
  });

  final PlaybackApi api;
  final String versionId;
  final List<TrickplayRef> refs;
  final Map<int, ui.Image> _cache = <int, ui.Image>{};
  final Set<int> _loading = <int>{};

  TrickplayRef? get _ref => refs.isEmpty ? null : refs.first;

  bool get available {
    final TrickplayRef? r = _ref;
    return r != null && r.intervalMs > 0 && r.columns > 0 && r.rows > 0;
  }

  int get sheetCount => refs.fold(
    0,
    (int most, TrickplayRef r) => r.sheets > most ? r.sheets : most,
  );

  ui.Image? sheetImage(int sheet, VoidCallback onLoaded) {
    final ui.Image? image = _cache[sheet];
    if (image == null) {
      _load(sheet, onLoaded);
    }
    return image;
  }

  TrickplayTile? tileFor(int positionMs, VoidCallback onLoaded) {
    final TrickplayRef? ref = _ref;
    if (ref == null) {
      return null;
    }
    final TrickplayCell? cell = trickplayCell(ref, positionMs);
    if (cell == null) {
      return null;
    }
    final ui.Image? image = _cache[cell.sheet];
    if (image == null) {
      _load(cell.sheet, onLoaded);
      return null;
    }
    return TrickplayTile(
      image: image,
      src: Rect.fromLTWH(
        (cell.col * ref.tileWidth).toDouble(),
        (cell.row * ref.tileHeight).toDouble(),
        ref.tileWidth.toDouble(),
        ref.tileHeight.toDouble(),
      ),
      aspect: ref.tileHeight > 0 ? ref.tileWidth / ref.tileHeight : 16 / 9,
    );
  }

  Future<void> _load(int sheet, VoidCallback onLoaded) async {
    if (sheet < 1 || _loading.contains(sheet) || _cache.containsKey(sheet)) {
      return;
    }
    final int most = sheetCount;
    if (most > 0 && sheet > most) {
      return;
    }
    _loading.add(sheet);
    try {
      final Uint8List bytes = await api.trickplaySheet(versionId, sheet);
      final ui.Codec codec = await ui.instantiateImageCodec(bytes);
      final ui.FrameInfo frame = await codec.getNextFrame();
      _cache[sheet] = frame.image;
      onLoaded();
    } catch (_) {
    } finally {
      _loading.remove(sheet);
    }
  }

  void dispose() {
    for (final ui.Image image in _cache.values) {
      image.dispose();
    }
    _cache.clear();
  }
}
