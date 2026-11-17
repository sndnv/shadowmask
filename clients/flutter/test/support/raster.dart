import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

Future<List<Color>> pixelsAt(
  WidgetTester tester,
  Finder boundary,
  List<Offset> points, {
  double ratio = 4,
}) async {
  final RenderRepaintBoundary render = tester.renderObject(boundary);
  final List<Color> out = <Color>[];
  await tester.runAsync(() async {
    final ui.Image image = await render.toImage(pixelRatio: ratio);
    final ByteData data = (await image.toByteData(
      format: ui.ImageByteFormat.rawStraightRgba,
    ))!;
    for (final Offset point in points) {
      final int x = (point.dx * ratio).floor();
      final int y = (point.dy * ratio).floor();
      final int at = (y * image.width + x) * 4;
      out.add(
        Color.fromARGB(
          data.getUint8(at + 3),
          data.getUint8(at),
          data.getUint8(at + 1),
          data.getUint8(at + 2),
        ),
      );
    }
    image.dispose();
  });
  return out;
}

Matcher isPixel(Color expected, {int tolerance = 2}) =>
    _IsPixel(expected, tolerance);

class _IsPixel extends Matcher {
  const _IsPixel(this.expected, this.tolerance);

  final Color expected;
  final int tolerance;

  int _channel(double value) => (value * 255).round();

  @override
  bool matches(Object? item, Map<dynamic, dynamic> matchState) {
    if (item is! Color) {
      return false;
    }
    return (_channel(item.a) - _channel(expected.a)).abs() <= tolerance &&
        (_channel(item.r) - _channel(expected.r)).abs() <= tolerance &&
        (_channel(item.g) - _channel(expected.g)).abs() <= tolerance &&
        (_channel(item.b) - _channel(expected.b)).abs() <= tolerance;
  }

  @override
  Description describe(Description description) => description.add(
    'a pixel within $tolerance of 0x${expected.toARGB32().toRadixString(16)}',
  );

  @override
  Description describeMismatch(
    Object? item,
    Description mismatchDescription,
    Map<dynamic, dynamic> matchState,
    bool verbose,
  ) => item is Color
      ? mismatchDescription.add('is 0x${item.toARGB32().toRadixString(16)}')
      : mismatchDescription.add('is not a colour');
}
