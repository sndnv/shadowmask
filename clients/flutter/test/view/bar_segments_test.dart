import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/view/bar_segments.dart';

void main() {
  test('six wide segments two apart fill a bar that fits them exactly', () {
    expect(barSegments(30), <({double left, double right})>[
      (left: 0, right: 6),
      (left: 8, right: 14),
      (left: 16, right: 22),
      (left: 24, right: 30),
    ]);
  });

  test('the last segment stops at the end of the bar', () {
    expect(barSegments(27).last, (left: 24, right: 27));
    expect(barSegments(25).last, (left: 24, right: 25));
  });

  test('a gap at the end starts no segment', () {
    expect(barSegments(23).last, (left: 16, right: 22));
    expect(barSegments(24).last, (left: 16, right: 22));
  });

  test('a bar narrower than a segment is one short segment', () {
    expect(barSegments(5), <({double left, double right})>[
      (left: 0, right: 5),
    ]);
  });

  test('a bar with no width has no segments', () {
    expect(barSegments(0), isEmpty);
  });
}
