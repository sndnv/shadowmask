import 'package:shadowmask/model/catalog/version_detail.dart';

class TrickplayCell {
  const TrickplayCell(this.sheet, this.row, this.col);

  final int sheet;
  final int row;
  final int col;
}

TrickplayCell? trickplayCell(TrickplayRef ref, int positionMs) {
  if (ref.intervalMs <= 0 || ref.columns <= 0 || ref.rows <= 0) {
    return null;
  }
  final int perSheet = ref.columns * ref.rows;
  final int idx = (positionMs < 0 ? 0 : positionMs) ~/ ref.intervalMs;
  final int cell = idx % perSheet;
  return TrickplayCell(
    idx ~/ perSheet + 1,
    cell ~/ ref.columns,
    cell % ref.columns,
  );
}
