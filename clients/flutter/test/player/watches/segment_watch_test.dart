import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/player/watches/segment_watch.dart';

bool lost(List<int> segments, {SegmentWatch from = kSegmentWatchArmed}) =>
    segmentsLost(segments.fold(from, segmentFailed));

void main() {
  test('a failed segment is read from either line mpv logs for it', () {
    expect(failedSegment('hls: Failed to open segment 42 of playlist 0'), 42);
    expect(
      failedSegment(
        'hls: Segment 7 of playlist 0 failed too many times, skipping',
      ),
      7,
    );
    expect(
      failedSegment('hls: Failed to open segment 1800 of playlist 2'),
      1800,
    );
    expect(failedSegment('Failed to open segment 42 of playlist 0'), 42);
    expect(failedSegment('http: HTTP error 410 Gone'), isNull);
    expect(failedSegment("hls: Opening 'seg_00042.ts' for reading"), isNull);
    expect(failedSegment('hls: Failed to open segment of playlist 0'), isNull);
    expect(
      failedSegment('Error: Failed to open segment 3 of playlist 0'),
      isNull,
    );
  });

  test('five segments failing in a row lose the stream', () {
    expect(lost(<int>[10, 11, 12, 13]), isFalse);
    expect(lost(<int>[10, 11, 12, 13, 14]), isTrue);
    expect(lost(<int>[0, 1, 2, 3, 4]), isTrue);
  });

  test('both lines for one segment count it once', () {
    expect(lost(<int>[10, 10, 11, 11, 12, 12, 13, 13]), isFalse);
    expect(lost(<int>[10, 10, 11, 11, 12, 12, 13, 13, 14, 14]), isTrue);
  });

  test('a lost stream is logged with the last segment that failed', () {
    expect(
      segmentsLostLine(42),
      'stream lost: 5 segments failed in a row, the last 42',
    );
  });

  test('a watch that is not armed never loses the stream', () {
    expect(
      lost(<int>[0, 1, 2, 3, 4, 5, 6, 7], from: kSegmentWatchStart),
      isFalse,
    );
    expect(segmentFailed(kSegmentWatchStart, 3), kSegmentWatchStart);
  });

  test('a segment that plays between failures starts the count again', () {
    expect(lost(<int>[10, 11, 12, 14, 15, 16, 17]), isFalse);
    expect(lost(<int>[10, 11, 12, 14, 15, 16, 17, 18]), isTrue);
    expect(lost(<int>[10, 11, 12, 13, 3, 4, 5, 6]), isFalse);
  });
}
