import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/page_director.dart';

PageDirector<String?> backdropsAfter(String? before) {
  final PageDirector<String?> backdrops = PageDirector<String?>(
    null,
    prepare: (String? _) => SynchronousFuture<bool>(true),
  );
  addTearDown(backdrops.dispose);
  if (before != null) {
    final Route<void> earlier = PageRouteBuilder<void>(
      pageBuilder: (BuildContext _, Animation<double> _, Animation<double> _) =>
          const SizedBox.shrink(),
    );
    backdrops.observer.didPush(earlier, null);
    backdrops.request(earlier, before);
  }
  return backdrops;
}

PageDirector<String?> pageTitles() {
  final PageDirector<String?> titles = PageDirector<String?>(null);
  addTearDown(titles.dispose);
  return titles;
}
