const int kFailedSegmentsInARow = 5;

typedef SegmentWatch = ({bool armed, int? last, int inARow});

const SegmentWatch kSegmentWatchStart = (armed: false, last: null, inARow: 0);

const SegmentWatch kSegmentWatchArmed = (armed: true, last: null, inARow: 0);

final RegExp _failedSegment = RegExp(
  r'^(?:hls: )?(?:Failed to open segment (\d+) of playlist \d+'
  r'|Segment (\d+) of playlist \d+ failed too many times)',
);

int? failedSegment(String text) {
  final RegExpMatch? match = _failedSegment.firstMatch(text);
  if (match == null) {
    return null;
  }
  return int.parse(match.group(1) ?? match.group(2)!);
}

SegmentWatch segmentFailed(SegmentWatch watch, int segment) =>
    switch (watch.last) {
      _ when !watch.armed => watch,
      final int last when segment == last => watch,
      final int last when segment == last + 1 => (
        armed: watch.armed,
        last: segment,
        inARow: watch.inARow + 1,
      ),
      _ => (armed: watch.armed, last: segment, inARow: 1),
    };

String segmentsLostLine(int last) =>
    'stream lost: $kFailedSegmentsInARow segments failed in a row, '
    'the last $last';

bool segmentsLost(SegmentWatch watch) => watch.inARow >= kFailedSegmentsInARow;
