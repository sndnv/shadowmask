import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/server/delivery_totals.dart';
import 'package:shadowmask/model/server/encoder_totals.dart';
import 'package:shadowmask/model/server/transcode_totals.dart';
import 'package:shadowmask/util/prometheus.dart';

const String _kExposition = '''
# TYPE build_info gauge
build_info{version="0.0.4"} 1
# TYPE transcode_segments_total counter
transcode_segments_total{encoder="software",outcome="ok"} 8
transcode_segments_total{encoder="software",outcome="failed"} 1
transcode_segments_total{encoder="vaapi",outcome="ok"} 20
transcode_segments_total{encoder="vaapi",outcome="failed"} 2
transcode_segments_total{encoder="remux",outcome="ok"} 5
transcode_segments_total{encoder="qsv",outcome="ok"} 1
# TYPE transcode_segment_duration_milliseconds histogram
transcode_segment_duration_milliseconds_bucket{encoder="vaapi",le="1000"} 15
transcode_segment_duration_milliseconds_bucket{encoder="vaapi",le="+Inf"} 20
transcode_segment_duration_milliseconds_sum{encoder="vaapi"} 20000
transcode_segment_duration_milliseconds_count{encoder="vaapi"} 20
transcode_segment_duration_milliseconds_sum{encoder="software"} 64000
transcode_segment_duration_milliseconds_count{encoder="software"} 8
transcode_media_milliseconds_total{encoder="vaapi"} 80000
transcode_media_milliseconds_total{encoder="software"} 32000
transcode_fallbacks_total 2
transcode_segments_abandoned_total 4
stream_starts_total{delivery="transcode",outcome="ok"} 3
stream_starts_total{delivery="transcode",outcome="failed"} 1
stream_starts_total{delivery="remux",outcome="ok"} 2
stream_first_segment_milliseconds_sum{delivery="transcode"} 9000
stream_first_segment_milliseconds_count{delivery="transcode"} 3
stream_keyframe_probe_duration_milliseconds_sum 6000
stream_keyframe_probe_duration_milliseconds_count 4
stream_keyframe_probe_failures_total 1
sessions_started_total{mode="direct"} 4
sessions_started_total{mode="transcode"} 5
stream_bytes_sent_total{kind="file"} 3000
stream_bytes_sent_total{kind="segment"} 2000
stream_bytes_sent_total{kind="playlist"} 10
''';

void main() {
  test('totals per encoder come in a fixed order, then the rest', () {
    final TranscodeTotals totals = TranscodeTotals.fromSamples(
      parseMetrics(_kExposition),
    );

    expect(totals.encoders.map((EncoderTotals e) => e.encoder), <String>[
      'vaapi',
      'software',
      'remux',
      'qsv',
    ]);
    final EncoderTotals vaapi = totals.encoders.first;
    expect(vaapi.produced, 20);
    expect(vaapi.failed, 2);
    expect(vaapi.timed, 20);
    expect(vaapi.averageMs, 1000);
    expect(vaapi.realtime, 4);
    final EncoderTotals software = totals.encoders[1];
    expect(software.averageMs, 8000);
    expect(software.realtime, 0.5);
    final EncoderTotals remux = totals.encoders[2];
    expect(remux.produced, 5);
    expect(remux.averageMs, isNull);
    expect(remux.realtime, isNull);
  });

  test('totals per delivery, keyframe reads, fallbacks and abandons', () {
    final TranscodeTotals totals = TranscodeTotals.fromSamples(
      parseMetrics(_kExposition),
    );

    expect(totals.deliveries.map((DeliveryTotals d) => d.delivery), <String>[
      'direct',
      'remux',
      'transcode',
    ]);
    final DeliveryTotals direct = totals.deliveries[0];
    final DeliveryTotals remux = totals.deliveries[1];
    final DeliveryTotals transcode = totals.deliveries[2];
    expect(direct.sessions, 4);
    expect(direct.streams, isFalse);
    expect(remux.sessions, 0);
    expect(remux.started, 2);
    expect(remux.streams, isTrue);
    expect(remux.averageFirstSegmentMs, isNull);
    expect(transcode.sessions, 5);
    expect(transcode.started, 3);
    expect(transcode.failed, 1);
    expect(transcode.averageFirstSegmentMs, 3000);
    expect(totals.fallbacks, 2);
    expect(totals.abandoned, 4);
    expect(totals.keyframeReads, 4);
    expect(totals.averageKeyframeMs, 1500);
    expect(totals.keyframeFailures, 1);
    expect(totals.directBytes, 3000);
    expect(totals.segmentBytes, 2000);
    expect(totals.isEmpty, isFalse);
  });

  test('a server that has streamed nothing has empty totals', () {
    final TranscodeTotals totals = TranscodeTotals.fromSamples(
      parseMetrics('build_info{version="0.0.4"} 1\n'),
    );

    expect(totals.isEmpty, isTrue);
    expect(totals.encoders, isEmpty);
    expect(totals.deliveries, isEmpty);
    expect(totals.averageKeyframeMs, isNull);
  });

  test('any single total makes the totals non-empty', () {
    for (final String line in <String>[
      'transcode_fallbacks_total 1',
      'transcode_segments_abandoned_total 1',
      'stream_keyframe_probe_duration_milliseconds_count 1',
      'stream_keyframe_probe_failures_total 1',
      'stream_bytes_sent_total{kind="file"} 1',
      'stream_bytes_sent_total{kind="segment"} 1',
      'stream_starts_total{delivery="remux",outcome="ok"} 1',
    ]) {
      expect(
        TranscodeTotals.fromSamples(parseMetrics(line)).isEmpty,
        isFalse,
        reason: line,
      );
    }
  });
}
