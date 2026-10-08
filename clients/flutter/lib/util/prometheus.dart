import 'dart:convert';

class MetricSample {
  const MetricSample(this.name, this.labels, this.value);

  final String name;
  final Map<String, String> labels;
  final double value;
}

List<MetricSample> parseMetrics(String text) => <MetricSample>[
  for (final String line in const LineSplitter().convert(text))
    ?_sample(line.trim()),
];

final RegExp _space = RegExp(r'\s');
final RegExp _spaces = RegExp(r'\s+');

MetricSample? _sample(String line) {
  if (line.isEmpty || line.startsWith('#')) {
    return null;
  }
  final int brace = line.indexOf('{');
  final int space = line.indexOf(_space);
  if (brace >= 0 && (space < 0 || brace < space)) {
    final ({Map<String, String> labels, int end})? parsed = _labels(
      line,
      brace + 1,
    );
    return parsed == null
        ? null
        : _valued(
            line.substring(0, brace),
            parsed.labels,
            line.substring(parsed.end),
          );
  }
  return space < 0
      ? null
      : _valued(
          line.substring(0, space),
          const <String, String>{},
          line.substring(space),
        );
}

MetricSample? _valued(String name, Map<String, String> labels, String rest) {
  final double? value = _number(rest.trim().split(_spaces).first);
  return name.isEmpty || value == null
      ? null
      : MetricSample(name, labels, value);
}

double? _number(String text) => switch (text) {
  '+Inf' => double.infinity,
  '-Inf' => double.negativeInfinity,
  _ => double.tryParse(text),
};

({Map<String, String> labels, int end})? _labels(String line, int start) {
  final Map<String, String> labels = <String, String>{};
  int i = _skipSeparators(line, start);
  while (i < line.length) {
    if (line[i] == '}') {
      return (labels: labels, end: i + 1);
    }
    final int eq = line.indexOf('=', i);
    if (eq < 0 || eq + 1 >= line.length || line[eq + 1] != '"') {
      return null;
    }
    final StringBuffer value = StringBuffer();
    int j = eq + 2;
    while (j < line.length && line[j] != '"') {
      if (line[j] == r'\' && j + 1 < line.length) {
        value.write(line[j + 1] == 'n' ? '\n' : line[j + 1]);
        j += 2;
      } else {
        value.write(line[j]);
        j += 1;
      }
    }
    if (j >= line.length) {
      return null;
    }
    labels[line.substring(i, eq).trim()] = value.toString();
    i = _skipSeparators(line, j + 1);
  }
  return null;
}

int _skipSeparators(String line, int start) {
  int i = start;
  while (i < line.length && (line[i] == ',' || line[i] == ' ')) {
    i += 1;
  }
  return i;
}
