import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/view/play_target.dart';

const List<Version> _versions = <Version>[
  Version(
    id: 'long',
    title: TitleRef(type: TitleKind.movie, id: 'm1'),
    libraryId: 'l1',
    quality: Quality.uhd,
    container: 'mkv',
    durationMs: 9000000,
  ),
  Version(
    id: 'short',
    title: TitleRef(type: TitleKind.movie, id: 'm1'),
    libraryId: 'l1',
    quality: Quality.fhd,
    container: 'mkv',
    durationMs: 6000000,
  ),
  Version(
    id: 'unknown',
    title: TitleRef(type: TitleKind.movie, id: 'm1'),
    libraryId: 'l1',
    quality: Quality.hd,
    container: 'mkv',
  ),
];

void main() {
  test('a switch carries the position to the other version', () {
    expect(switchStartMs(_versions, 'short', 61000, ended: false), 61000);
    expect(switchStartMs(_versions, 'unknown', 7000000, ended: false), 7000000);
  });

  test('a switch after the end starts the other version from the top', () {
    expect(switchStartMs(_versions, 'long', 6000000, ended: true), 0);
  });

  test('a switch past the other version\'s length starts it from the top', () {
    expect(switchStartMs(_versions, 'short', 7000000, ended: false), 0);
    expect(switchStartMs(_versions, 'short', 6000000, ended: false), 0);
    expect(switchStartMs(_versions, 'long', 7000000, ended: false), 7000000);
  });
}
