import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';

void main() {
  group('SubtitleSelection codec', () {
    test('embedded round trips through the URL form', () {
      final SubtitleSelection s = SubtitleSelection.fromWire('embedded:5')!;
      expect(s.kind, SubtitleKind.embedded);
      expect(s.index, 5);
      expect(s.toWire(), 'embedded:5');
    });

    test('file round trips through the URL form', () {
      final SubtitleSelection s = SubtitleSelection.fromWire('file:abc')!;
      expect(s.kind, SubtitleKind.file);
      expect(s.id, 'abc');
      expect(s.toWire(), 'file:abc');
    });

    test('empty or null is no selection', () {
      expect(SubtitleSelection.fromWire(''), isNull);
      expect(SubtitleSelection.fromWire(null), isNull);
    });

    test('toJson matches the protocol track shape', () {
      expect(const SubtitleSelection.embedded(5).toJson(), <String, dynamic>{
        'type': 'embedded',
        'index': 5,
      });
      expect(const SubtitleSelection.file('x').toJson(), <String, dynamic>{
        'type': 'file',
        'id': 'x',
      });
    });
  });
}
