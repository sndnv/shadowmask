import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_state.dart';

typedef PlaybackStep = (PlaybackState, List<PlaybackCommand>);

typedef PlaybackWrite = PlaybackStep Function(PlaybackState);
