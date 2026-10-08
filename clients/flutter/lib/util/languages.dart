import 'package:shadowmask/l10n/strings.dart';

const Map<String, String> kLanguages = <String, String>{
  'af': 'Afrikaans',
  'sq': 'Albanian',
  'am': 'Amharic',
  'ar': 'Arabic',
  'hy': 'Armenian',
  'az': 'Azerbaijani',
  'eu': 'Basque',
  'be': 'Belarusian',
  'bn': 'Bengali',
  'bs': 'Bosnian',
  'bg': 'Bulgarian',
  'my': 'Burmese',
  'ca': 'Catalan',
  'zh': 'Chinese',
  'zh-CN': 'Chinese (Simplified)',
  'zh-TW': 'Chinese (Traditional)',
  'hr': 'Croatian',
  'cs': 'Czech',
  'da': 'Danish',
  'nl': 'Dutch',
  'en': 'English',
  'eo': 'Esperanto',
  'et': 'Estonian',
  'fil': 'Filipino',
  'fi': 'Finnish',
  'fr': 'French',
  'gl': 'Galician',
  'ka': 'Georgian',
  'de': 'German',
  'el': 'Greek',
  'he': 'Hebrew',
  'hi': 'Hindi',
  'hu': 'Hungarian',
  'is': 'Icelandic',
  'id': 'Indonesian',
  'ga': 'Irish',
  'it': 'Italian',
  'ja': 'Japanese',
  'kn': 'Kannada',
  'kk': 'Kazakh',
  'km': 'Khmer',
  'ko': 'Korean',
  'ku': 'Kurdish',
  'lo': 'Lao',
  'la': 'Latin',
  'lv': 'Latvian',
  'lt': 'Lithuanian',
  'mk': 'Macedonian',
  'ms': 'Malay',
  'ml': 'Malayalam',
  'mt': 'Maltese',
  'mr': 'Marathi',
  'mn': 'Mongolian',
  'ne': 'Nepali',
  'no': 'Norwegian',
  'nb': 'Norwegian Bokmål',
  'nn': 'Norwegian Nynorsk',
  'fa': 'Persian',
  'pl': 'Polish',
  'pt': 'Portuguese',
  'pt-BR': 'Portuguese (Brazil)',
  'pt-PT': 'Portuguese (Portugal)',
  'pa': 'Punjabi',
  'ro': 'Romanian',
  'ru': 'Russian',
  'sr': 'Serbian',
  'si': 'Sinhala',
  'sk': 'Slovak',
  'sl': 'Slovenian',
  'so': 'Somali',
  'es': 'Spanish',
  'sw': 'Swahili',
  'sv': 'Swedish',
  'tl': 'Tagalog',
  'ta': 'Tamil',
  'te': 'Telugu',
  'th': 'Thai',
  'tr': 'Turkish',
  'uk': 'Ukrainian',
  'ur': 'Urdu',
  'uz': 'Uzbek',
  'vi': 'Vietnamese',
  'cy': 'Welsh',
  'yi': 'Yiddish',
  'zu': 'Zulu',
};

const String _kUndetermined = 'und';

final RegExp _regional = RegExp(r'^([a-zA-Z]{2})[-_]([a-zA-Z]{2})$');

String _key(String code) {
  final RegExpMatch? regional = _regional.firstMatch(code);
  return regional == null
      ? code.toLowerCase()
      : '${regional[1]!.toLowerCase()}-${regional[2]!.toUpperCase()}';
}

Iterable<String> languageParts(String code) => code
    .split('+')
    .map(_key)
    .where((String part) => part.isNotEmpty && part != _kUndetermined);

String languageLabel(String code) {
  if (code.contains('+')) {
    return code.split('+').map(languageLabel).join(' + ');
  }
  final String key = _key(code);
  if (key == _kUndetermined) {
    return Strings.unknownValue;
  }
  final String? known = kLanguages[key];
  if (known != null) {
    return known;
  }
  final RegExpMatch? regional = _regional.firstMatch(key);
  if (regional != null) {
    final String? base = kLanguages[regional[1]];
    if (base != null) {
      return '$base (${regional[2]!})';
    }
  }
  return code.toUpperCase();
}

List<(String, String)> languageOptions({String? keep}) {
  final List<(String, String)> options = kLanguages.entries
      .map((MapEntry<String, String> e) => (e.key, e.value))
      .toList();
  final String? extra = keep == null ? null : _key(keep);
  if (extra != null && extra.isNotEmpty && !kLanguages.containsKey(extra)) {
    options.add((keep!, languageLabel(keep)));
  }
  options.sort(
    ((String, String) a, (String, String) b) => a.$2.compareTo(b.$2),
  );
  return options;
}
