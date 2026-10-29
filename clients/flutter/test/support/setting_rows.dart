import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/theme/space.dart';

void expectSettingRow(WidgetTester tester, String title, String help) {
  final Rect row = tester.getRect(find.widgetWithText(ListTile, title));
  final Rect name = tester.getRect(find.text(title));
  final Rect hint = tester.getRect(find.text(help));
  expect(name.left - row.left, moreOrLessEquals(Space.s2), reason: title);
  expect(hint.left - name.left, moreOrLessEquals(Space.s2), reason: help);
  expect(name.top - row.top, moreOrLessEquals(Space.s3), reason: title);
  expect(hint.top - name.bottom, moreOrLessEquals(Space.s1), reason: help);
  expect(row.bottom - hint.bottom, moreOrLessEquals(Space.s3), reason: help);
}
