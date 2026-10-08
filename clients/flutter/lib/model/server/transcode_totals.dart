import 'package:shadowmask/model/server/delivery_totals.dart';
import 'package:shadowmask/model/server/encoder_totals.dart';
import 'package:shadowmask/util/prometheus.dart';

const List<String> _kEncoderOrder = <String>['vaapi', 'software', 'remux'];
const List<String> _kDeliveryOrder = <String>['direct', 'remux', 'transcode'];

class TranscodeTotals {
  const TranscodeTotals({
    this.encoders = const <EncoderTotals>[],
    this.deliveries = const <DeliveryTotals>[],
    this.fallbacks = 0,
    this.abandoned = 0,
    this.keyframeReads = 0,
    this.keyframeMs = 0,
    this.keyframeFailures = 0,
    this.directBytes = 0,
    this.segmentBytes = 0,
  });

  factory TranscodeTotals.fromSamples(List<MetricSample> samples) {
    double total(
      String name, [
      Map<String, String> match = const <String, String>{},
    ]) => samples
        .where(
          (MetricSample s) =>
              s.name == name &&
              match.entries.every(
                (MapEntry<String, String> e) => s.labels[e.key] == e.value,
              ),
        )
        .fold<double>(0, (double sum, MetricSample s) => sum + s.value);
    List<String> seen(List<(String, String)> sources, List<String> order) {
      final Set<String> found = <String>{
        for (final MetricSample s in samples)
          for (final (String name, String label) in sources)
            if (s.name == name) ?s.labels[label],
      };
      final List<String> rest =
          found.where((String v) => !order.contains(v)).toList()..sort();
      return <String>[...order.where(found.contains), ...rest];
    }

    return TranscodeTotals(
      encoders: <EncoderTotals>[
        for (final String e in seen(const <(String, String)>[
          ('transcode_segments_total', 'encoder'),
          ('transcode_segment_duration_milliseconds_count', 'encoder'),
          ('transcode_media_milliseconds_total', 'encoder'),
        ], _kEncoderOrder))
          EncoderTotals(
            encoder: e,
            produced: total('transcode_segments_total', <String, String>{
              'encoder': e,
              'outcome': 'ok',
            }).round(),
            failed: total('transcode_segments_total', <String, String>{
              'encoder': e,
              'outcome': 'failed',
            }).round(),
            timeMs: total(
              'transcode_segment_duration_milliseconds_sum',
              <String, String>{'encoder': e},
            ),
            timed: total(
              'transcode_segment_duration_milliseconds_count',
              <String, String>{'encoder': e},
            ).round(),
            mediaMs: total(
              'transcode_media_milliseconds_total',
              <String, String>{'encoder': e},
            ),
          ),
      ],
      deliveries: <DeliveryTotals>[
        for (final String d in seen(const <(String, String)>[
          ('sessions_started_total', 'mode'),
          ('stream_starts_total', 'delivery'),
          ('stream_first_segment_milliseconds_count', 'delivery'),
        ], _kDeliveryOrder))
          DeliveryTotals(
            delivery: d,
            sessions: total('sessions_started_total', <String, String>{
              'mode': d,
            }).round(),
            started: total('stream_starts_total', <String, String>{
              'delivery': d,
              'outcome': 'ok',
            }).round(),
            failed: total('stream_starts_total', <String, String>{
              'delivery': d,
              'outcome': 'failed',
            }).round(),
            firstSegmentMs: total(
              'stream_first_segment_milliseconds_sum',
              <String, String>{'delivery': d},
            ),
            firstSegments: total(
              'stream_first_segment_milliseconds_count',
              <String, String>{'delivery': d},
            ).round(),
          ),
      ],
      fallbacks: total('transcode_fallbacks_total').round(),
      abandoned: total('transcode_segments_abandoned_total').round(),
      keyframeReads: total(
        'stream_keyframe_probe_duration_milliseconds_count',
      ).round(),
      keyframeMs: total('stream_keyframe_probe_duration_milliseconds_sum'),
      keyframeFailures: total('stream_keyframe_probe_failures_total').round(),
      directBytes: total('stream_bytes_sent_total', <String, String>{
        'kind': 'file',
      }).round(),
      segmentBytes: total('stream_bytes_sent_total', <String, String>{
        'kind': 'segment',
      }).round(),
    );
  }

  final List<EncoderTotals> encoders;
  final List<DeliveryTotals> deliveries;
  final int fallbacks;
  final int abandoned;
  final int keyframeReads;
  final double keyframeMs;
  final int keyframeFailures;
  final int directBytes;
  final int segmentBytes;

  bool get isEmpty =>
      encoders.isEmpty &&
      deliveries.isEmpty &&
      fallbacks == 0 &&
      abandoned == 0 &&
      keyframeReads == 0 &&
      keyframeFailures == 0 &&
      directBytes == 0 &&
      segmentBytes == 0;

  double? get averageKeyframeMs =>
      keyframeReads == 0 ? null : keyframeMs / keyframeReads;
}
