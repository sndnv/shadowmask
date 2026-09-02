import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/title_ref.dart';

void main() {
  test('title ref round-trips to the wire shape', () {
    expect(
      const TitleRef(type: TitleKind.episode, id: 'e1').toJson(),
      <String, dynamic>{'type': 'episode', 'id': 'e1'},
    );
  });
}
