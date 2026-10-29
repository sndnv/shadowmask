import 'dart:math' as math;

import 'package:shadowmask/l10n/strings.dart';

enum DialogueLift {
  none(0, Strings.dialogueNormal),
  db3(3, Strings.dialogueDb3),
  db6(6, Strings.dialogueDb6),
  db9(9, Strings.dialogueDb9);

  const DialogueLift(this.db, this.label);

  final int db;
  final String label;

  double get othersGain => math.pow(10, -db / 20).toDouble();

  static DialogueLift fromDb(int? value) => values.firstWhere(
    (DialogueLift lift) => lift.db == value,
    orElse: () => DialogueLift.none,
  );
}

enum NightMode {
  off('off', Strings.nightOff),
  light('light', Strings.nightLight),
  strong('strong', Strings.nightStrong);

  const NightMode(this.wire, this.label);

  final String wire;
  final String label;

  static NightMode fromWire(String? value) => values.firstWhere(
    (NightMode mode) => mode.wire == value,
    orElse: () => NightMode.off,
  );
}

Map<String, String> mpvSound({
  required bool normalize,
  required DialogueLift dialogue,
}) => <String, String>{
  'audio-normalize-downmix': normalize ? 'yes' : 'no',
  'audio-swresample-o': dialogue == DialogueLift.none
      ? ''
      : 'center_mix_level='
            '${(math.sqrt1_2 / dialogue.othersGain).toStringAsFixed(6)}',
};

typedef SoundRows = ({bool downmix, bool normalize, bool dialogue, bool night});

const SoundRows kNoSoundRows = (
  downmix: false,
  normalize: false,
  dialogue: false,
  night: false,
);

SoundRows soundRows({
  required int? source,
  required int? sent,
  required int? output,
  required bool mixesItself,
  required DialogueLift dialogue,
}) {
  if (source == null || source < 1) {
    return kNoSoundRows;
  }
  final int received = sent ?? source;
  final bool playerFolds =
      mixesItself && received > 2 && (output == null || output < received);
  return (
    downmix: source > 2,
    normalize:
        normalizeReachesServer(
          source: source,
          sent: sent,
          dialogue: dialogue,
        ) ||
        playerFolds,
    dialogue:
        liftReachesServer(
          source: source,
          sent: sent,
          mixesItself: mixesItself,
        ) ||
        playerFolds,
    night: true,
  );
}

bool normalizeReachesServer({
  required int source,
  required int? sent,
  required DialogueLift dialogue,
}) =>
    (sent ?? source) < source || (source == 2 && dialogue != DialogueLift.none);

bool liftReachesServer({
  required int source,
  required int? sent,
  required bool mixesItself,
}) => source == 2 || (source > 2 && (!mixesItself || (sent ?? source) <= 2));
