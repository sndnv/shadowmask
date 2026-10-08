import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/prometheus.dart';

void main() {
  test('reads names, labels and values, skipping comments and blanks', () {
    final List<MetricSample> samples = parseMetrics(
      '# HELP build_info Build.\n'
      '# TYPE build_info gauge\n'
      'build_info{version="0.0.4"} 1\n'
      '\n'
      'transcode_fallbacks_total 3\n'
      'transcode_segments_total{encoder="vaapi",outcome="ok"} 12\n',
    );

    expect(samples, hasLength(3));
    expect(samples[0].name, 'build_info');
    expect(samples[0].labels, <String, String>{'version': '0.0.4'});
    expect(samples[0].value, 1);
    expect(samples[1].name, 'transcode_fallbacks_total');
    expect(samples[1].labels, isEmpty);
    expect(samples[1].value, 3);
    expect(samples[2].labels, <String, String>{
      'encoder': 'vaapi',
      'outcome': 'ok',
    });
    expect(samples[2].value, 12);
  });

  test('unescapes label values', () {
    final MetricSample sample = parseMetrics(
      r'm{a="say \"hi\"",b="back\\slash",c="two\nlines", d="x"} 2',
    ).single;

    expect(sample.labels, <String, String>{
      'a': 'say "hi"',
      'b': r'back\slash',
      'c': 'two\nlines',
      'd': 'x',
    });
  });

  test('a label value may hold braces, commas and spaces', () {
    final MetricSample sample = parseMetrics('m{a="x} {y, z"} 4').single;

    expect(sample.labels, <String, String>{'a': 'x} {y, z'});
    expect(sample.value, 4);
  });

  test('reads infinities, NaN, decimals and ignores a timestamp', () {
    final List<MetricSample> samples = parseMetrics(
      'a +Inf\n'
      'b -Inf\n'
      'c NaN\n'
      'd 1.5e3 1700000000000\n'
      'e{} 0.25\n',
    );

    expect(samples.map((MetricSample s) => s.name), <String>[
      'a',
      'b',
      'c',
      'd',
      'e',
    ]);
    expect(samples[0].value, double.infinity);
    expect(samples[1].value, double.negativeInfinity);
    expect(samples[2].value.isNaN, isTrue);
    expect(samples[3].value, 1500);
    expect(samples[4].labels, isEmpty);
    expect(samples[4].value, 0.25);
  });

  test('skips lines it cannot read', () {
    final List<MetricSample> samples = parseMetrics(
      'no_value\n'
      'no_value_after_labels{a="b"}\n'
      'not_a_number abc\n'
      'unterminated{a="b} 1\n'
      'unquoted{a=b} 1\n'
      'no_equals{a} 1\n'
      'equals_at_end{a=\n'
      'unclosed{a="b" 1\n'
      '{a="b"} 1\n'
      'good 7\n',
    );

    expect(samples.single.name, 'good');
    expect(samples.single.value, 7);
  });
}
