import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/util/languages.dart';

void main() {
  test('a two-letter code reads as its language', () {
    expect(languageLabel('en'), 'English');
    expect(languageLabel('FR'), 'French');
  });

  test('a code the table does not know reads as itself', () {
    expect(languageLabel('eng'), 'ENG');
  });

  test('both Norwegian written forms have names', () {
    expect(languageLabel('nb'), 'Norwegian Bokmål');
    expect(languageLabel('nn'), 'Norwegian Nynorsk');
  });

  test('an undetermined language reads as unknown', () {
    expect(languageLabel('und'), Strings.unknownValue);
    expect(languageLabel('UND'), Strings.unknownValue);
  });

  test('a combined code reads as its parts', () {
    expect(languageLabel('en+fr'), 'English + French');
    expect(languageLabel('en+und'), 'English + ${Strings.unknownValue}');
    expect(
      languageLabel('pt-BR+zh-TW'),
      'Portuguese (Brazil) + Chinese (Traditional)',
    );
  });

  test('the parts of a code leave out the undetermined ones', () {
    expect(languageParts('en'), <String>['en']);
    expect(languageParts('EN+pt_br'), <String>['en', 'pt-BR']);
    expect(languageParts('und+ja'), <String>['ja']);
    expect(languageParts('und'), isEmpty);
    expect(languageParts('en+'), <String>['en']);
  });

  test('a listed region reads as its own name, in any case or separator', () {
    expect(languageLabel('pt-BR'), 'Portuguese (Brazil)');
    expect(languageLabel('pt-PT'), 'Portuguese (Portugal)');
    expect(languageLabel('zh-CN'), 'Chinese (Simplified)');
    expect(languageLabel('zh-TW'), 'Chinese (Traditional)');
    expect(languageLabel('pt-br'), 'Portuguese (Brazil)');
    expect(languageLabel('pt_BR'), 'Portuguese (Brazil)');
    expect(languageLabel('PT_br'), 'Portuguese (Brazil)');
  });

  test('a region the table does not list reads as its language and region', () {
    expect(languageLabel('en-US'), 'English (US)');
    expect(languageLabel('fr_ca'), 'French (CA)');
  });

  test('a region of a language the table does not know reads as itself', () {
    expect(languageLabel('xx-YY'), 'XX-YY');
  });

  test(
    'every code is two letters, a two-letter code with a region, or fil',
    () {
      expect(
        kLanguages.keys.where(
          (String code) =>
              !RegExp(r'^[a-z]{2}$').hasMatch(code) &&
              !RegExp(r'^[a-z]{2}-[A-Z]{2}$').hasMatch(code),
        ),
        <String>['fil'],
      );
    },
  );

  test('the options list every language once, sorted by name', () {
    final List<(String, String)> options = languageOptions();
    expect(options.length, kLanguages.length);
    final List<String> labels = options
        .map(((String, String) o) => o.$2)
        .toList();
    expect(labels, List<String>.of(labels)..sort());
    expect(labels.where((String l) => l.startsWith('Chinese')), <String>[
      'Chinese',
      'Chinese (Simplified)',
      'Chinese (Traditional)',
    ]);
  });

  test('a kept listed code adds nothing, in any case', () {
    expect(languageOptions(keep: 'pt-BR').length, kLanguages.length);
    expect(languageOptions(keep: 'pt-br').length, kLanguages.length);
    expect(languageOptions(keep: '').length, kLanguages.length);
  });

  test('a kept unlisted code is added with its label', () {
    final List<(String, String)> options = languageOptions(keep: 'en-US');
    expect(options.length, kLanguages.length + 1);
    expect(options, contains(('en-US', 'English (US)')));
  });
}
