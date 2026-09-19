import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/view/timeline_marks.dart';

void main() {
  group('TimelineMarks', () {
    test('without a preview everything sits on the playback position', () {
      final TimelineMarks marks = TimelineMarks.of(width: 400, fraction: 0.5);
      expect(marks.fillTo, 200);
      expect(marks.previewTo, 200);
      expect(marks.thumbAt, 200);
      expect(marks.ghostAt, isNull);
      expect(marks.hasPreview, isFalse);
    });

    // The accent stops where playback actually is; the span the viewer is
    // about to cross is the muted one, which is what makes the jump readable.
    test('a forward preview leaves the accent behind and mutes the gap', () {
      final TimelineMarks marks = TimelineMarks.of(
        width: 400,
        fraction: 0.25,
        preview: 0.75,
      );
      expect(marks.fillTo, 100);
      expect(marks.previewTo, 300);
      expect(marks.thumbAt, 100);
      expect(marks.ghostAt, 300);
      expect(marks.hasPreview, isTrue);
    });

    // Jumping back mutes the stretch about to be given up, so the muted span
    // always describes the jump rather than the direction.
    test('a backward preview mutes the stretch between target and now', () {
      final TimelineMarks marks = TimelineMarks.of(
        width: 400,
        fraction: 0.75,
        preview: 0.25,
      );
      expect(marks.fillTo, 100);
      expect(marks.previewTo, 300);
      expect(marks.thumbAt, 300);
      expect(marks.ghostAt, 100);
      expect(marks.hasPreview, isTrue);
    });

    test('a preview on the playback position paints no span', () {
      final TimelineMarks marks = TimelineMarks.of(
        width: 400,
        fraction: 0.5,
        preview: 0.5,
      );
      expect(marks.hasPreview, isFalse);
      expect(marks.ghostAt, 200);
    });

    test('both positions are clamped to the bar', () {
      final TimelineMarks marks = TimelineMarks.of(
        width: 400,
        fraction: 1.4,
        preview: -0.3,
      );
      expect(marks.fillTo, 0);
      expect(marks.previewTo, 400);
      expect(marks.thumbAt, 400);
      expect(marks.ghostAt, 0);
    });

    test('equality covers every mark', () {
      const TimelineMarks base = TimelineMarks(
        fillTo: 1,
        previewTo: 2,
        thumbAt: 3,
        ghostAt: 4,
      );
      expect(
        base,
        const TimelineMarks(fillTo: 1, previewTo: 2, thumbAt: 3, ghostAt: 4),
      );
      expect(
        base.hashCode,
        const TimelineMarks(
          fillTo: 1,
          previewTo: 2,
          thumbAt: 3,
          ghostAt: 4,
        ).hashCode,
      );
      for (final TimelineMarks other in <TimelineMarks>[
        const TimelineMarks(fillTo: 9, previewTo: 2, thumbAt: 3, ghostAt: 4),
        const TimelineMarks(fillTo: 1, previewTo: 9, thumbAt: 3, ghostAt: 4),
        const TimelineMarks(fillTo: 1, previewTo: 2, thumbAt: 9, ghostAt: 4),
        const TimelineMarks(fillTo: 1, previewTo: 2, thumbAt: 3, ghostAt: null),
      ]) {
        expect(base, isNot(other));
      }
    });
  });
}
