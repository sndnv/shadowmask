import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/player/trickplay_cell.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';

void main() {
  group('trickplayCell', () {
    const TrickplayRef ref = TrickplayRef(
      intervalMs: 10000,
      columns: 5,
      rows: 5,
      tileWidth: 320,
      tileHeight: 180,
      sheets: 2,
    );

    test('the first tile is top-left of the first sheet', () {
      final TrickplayCell c = trickplayCell(ref, 0)!;
      expect(c.sheet, 1);
      expect(c.row, 0);
      expect(c.col, 0);
    });

    test('a mid-sheet time maps to the right row and column', () {
      final TrickplayCell c = trickplayCell(ref, 70000)!;
      expect(c.sheet, 1);
      expect(c.row, 1);
      expect(c.col, 2);
    });

    test('spills onto the second sheet past the grid', () {
      final TrickplayCell c = trickplayCell(ref, 260000)!;
      expect(c.sheet, 2);
      expect(c.row, 0);
      expect(c.col, 1);
    });

    test('an empty ref has no cell', () {
      expect(trickplayCell(const TrickplayRef(), 1000), isNull);
    });
  });
}
