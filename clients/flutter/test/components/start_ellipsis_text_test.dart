import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/start_ellipsis_text.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

const String _name =
    'Paper.Skies.Part.II.2020.PROPER.1080p.WEB-DL.DDP5.1.Atmos.H264-GRP.mkv';

Future<String> _shown(WidgetTester tester, Widget child, double width) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: Center(
          child: SizedBox(width: width, child: child),
        ),
      ),
    ),
  );
  return tester.widget<Text>(find.byType(Text)).data!;
}

void main() {
  testWidgets('a name that fits is shown whole in either mode', (
    WidgetTester tester,
  ) async {
    expect(
      await _shown(tester, const StartEllipsisText('short.mkv'), 400),
      'short.mkv',
    );
    expect(
      await _shown(tester, const StartEllipsisText.middle('short.mkv'), 400),
      'short.mkv',
    );
  });

  testWidgets('the start mode keeps the end of the text', (
    WidgetTester tester,
  ) async {
    final String shown = await _shown(
      tester,
      const StartEllipsisText(_name),
      160,
    );

    expect(shown, startsWith('…'));
    expect(_name, endsWith(shown.substring(1)));
  });

  testWidgets('the middle mode keeps the start and the extension', (
    WidgetTester tester,
  ) async {
    final String shown = await _shown(
      tester,
      const StartEllipsisText.middle(_name),
      160,
    );
    final List<String> halves = shown.split('…');

    expect(halves, hasLength(2));
    expect(halves.first, isNotEmpty);
    expect(_name, startsWith(halves.first));
    expect(halves.last, endsWith('.mkv'));
    expect(_name, endsWith(halves.last));
    expect(shown.length, lessThan(_name.length));
  });

  testWidgets('the middle mode never shows more than the space allows', (
    WidgetTester tester,
  ) async {
    for (final double width in <double>[40, 90, 160, 240]) {
      await _shown(tester, const StartEllipsisText.middle(_name), width);

      expect(tester.takeException(), isNull, reason: 'at $width');
      expect(
        tester.getSize(find.byType(Text)).width,
        lessThanOrEqualTo(width),
        reason: 'at $width',
      );
    }
  });

  testWidgets('the tooltip carries the full text when one is given', (
    WidgetTester tester,
  ) async {
    await _shown(
      tester,
      const StartEllipsisText.middle(_name, tooltip: '/media/movies/$_name'),
      160,
    );

    expect(
      tester.widget<Tooltip>(find.byType(Tooltip)).message,
      '/media/movies/$_name',
    );
  });
}
