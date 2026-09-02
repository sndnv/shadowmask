import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/model/library/scan_state.dart';

String libraryKindLabel(LibraryKind kind) =>
    kind == LibraryKind.movie ? 'Movie' : 'TV';

String libraryKindWire(LibraryKind kind) =>
    kind == LibraryKind.movie ? 'movie' : 'tv';

String libraryOriginLabel(LibraryOrigin origin) =>
    origin == LibraryOrigin.local ? 'Local' : 'External';

String libraryOriginWire(LibraryOrigin origin) =>
    origin == LibraryOrigin.local ? 'local' : 'external';

String watcherLabel(WatcherStrategy watcher) => switch (watcher) {
  WatcherStrategy.local => 'Local',
  WatcherStrategy.polling => 'Polling',
  WatcherStrategy.scheduled => 'Scheduled',
  WatcherStrategy.manual => 'Manual',
};

String watcherWire(WatcherStrategy watcher) => switch (watcher) {
  WatcherStrategy.local => 'local',
  WatcherStrategy.polling => 'polling',
  WatcherStrategy.scheduled => 'scheduled',
  WatcherStrategy.manual => 'manual',
};

String scanStatusLabel(ScanStatus status) => switch (status) {
  ScanStatus.idle => 'Idle',
  ScanStatus.queued => 'Queued',
  ScanStatus.running => 'Running',
  ScanStatus.failed => 'Failed',
};
