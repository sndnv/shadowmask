import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/languages.dart';

void main() {
  test('a two-letter code reads as its language', () {
    expect(languageLabel('en'), 'English');
    expect(languageLabel('FR'), 'French');
  });

  test('a code the table does not know reads as itself', () {
    expect(languageLabel('eng'), 'ENG');
  });

  test('every code is two letters except one without a two-letter form', () {
    expect(kLanguages.keys.where((String code) => code.length != 2), <String>[
      'fil',
    ]);
  });
}
