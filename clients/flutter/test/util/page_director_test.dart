import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/page_director.dart';

Route<void> _page() => PageRouteBuilder<void>(
  pageBuilder: (BuildContext _, Animation<double> _, Animation<double> _) =>
      const SizedBox(),
);

Route<void> _dialog() => RawDialogRoute<void>(
  pageBuilder: (BuildContext _, Animation<double> _, Animation<double> _) =>
      const SizedBox(),
);

Future<void> _frame(WidgetTester tester) async {
  tester.binding.scheduleFrame();
  await tester.pump();
}

class _Images {
  final Map<String?, Completer<bool>> _loads = <String?, Completer<bool>>{};

  Future<bool> prepare(String? url) =>
      (_loads[url] ??= Completer<bool>()).future;

  void ready(String url) => (_loads[url] ??= Completer<bool>()).complete(true);

  void fail(String url) => (_loads[url] ??= Completer<bool>()).complete(false);
}

class _Stage {
  _Stage(this.director);

  final PageDirector<String?> director;
  final List<Route<dynamic>> _routes = <Route<dynamic>>[];
  final List<String?> seen = <String?>[];
  int told = 0;

  Route<dynamic> get top => _routes.last;

  Route<dynamic> push(Route<dynamic> route) {
    director.observer.didPush(route, _routes.isEmpty ? null : _routes.last);
    _routes.add(route);
    return route;
  }

  void pop() {
    final Route<dynamic> leaving = _routes.removeLast();
    director.observer.didPop(leaving, _routes.isEmpty ? null : _routes.last);
  }

  void replace(Route<dynamic> next) {
    final Route<dynamic> old = _routes.removeLast();
    director.observer.didReplace(newRoute: next, oldRoute: old);
    _routes.add(next);
  }

  void remove(Route<dynamic> route) {
    final int at = _routes.indexOf(route);
    _routes.removeAt(at);
    director.observer.didRemove(
      route,
      at == 0 || _routes.isEmpty ? null : _routes[at - 1],
    );
  }
}

_Stage _stage(_Images images, {String? initial}) {
  final PageDirector<String?> director = PageDirector<String?>(
    initial,
    prepare: images.prepare,
  );
  addTearDown(director.dispose);
  final _Stage stage = _Stage(director);
  director.addListener(() {
    stage.told++;
    stage.seen.add(director.shown);
  });
  return stage;
}

Future<_Stage> _showing(WidgetTester tester, _Images images, String url) async {
  final _Stage stage = _stage(images);
  stage.director.request(stage.push(_page()), url);
  images.ready(url);
  await _frame(tester);
  expect(stage.director.shown, url);
  stage.told = 0;
  stage.seen.clear();
  return stage;
}

void main() {
  testWidgets('before any page asks, the initial value is shown', (
    WidgetTester tester,
  ) async {
    final _Stage stage = _stage(_Images(), initial: 'start');

    stage.push(_page());
    await _frame(tester);

    expect(stage.director.shown, 'start');
    expect(stage.told, 0);
  });

  testWidgets('the page on screen is shown once its image is ready', (
    WidgetTester tester,
  ) async {
    final _Images images = _Images();
    final _Stage stage = _stage(images);

    stage.director.request(stage.push(_page()), 'a');
    await _frame(tester);
    expect(stage.director.shown, isNull);

    images.ready('a');
    await _frame(tester);
    expect(stage.director.shown, 'a');
    expect(stage.told, 1);
  });

  testWidgets('a page pushed over another keeps what is shown until it asks', (
    WidgetTester tester,
  ) async {
    final _Stage stage = await _showing(tester, _Images(), 'a');

    stage.push(_page());
    await _frame(tester);

    expect(stage.director.shown, 'a');
    expect(stage.told, 0);
  });

  testWidgets('a fallback of none clears what is shown at once', (
    WidgetTester tester,
  ) async {
    final _Stage stage = await _showing(tester, _Images(), 'a');

    stage.director.fallback(stage.push(_page()), null);
    await _frame(tester);

    expect(stage.director.shown, isNull);
  });

  testWidgets('a different value is shown only once it is ready', (
    WidgetTester tester,
  ) async {
    final _Images images = _Images();
    final _Stage stage = await _showing(tester, images, 'a');

    stage.director.request(stage.push(_page()), 'b');
    await _frame(tester);
    expect(stage.director.shown, 'a');

    images.ready('b');
    await _frame(tester);
    expect(stage.director.shown, 'b');
    expect(stage.told, 1);
  });

  testWidgets('a page asking for what is shown tells no one', (
    WidgetTester tester,
  ) async {
    final _Stage stage = await _showing(tester, _Images(), 'a');

    stage.director.request(stage.push(_page()), 'a');
    await _frame(tester);

    expect(stage.director.shown, 'a');
    expect(stage.told, 0);
  });

  testWidgets('back, the revealed page is shown again', (
    WidgetTester tester,
  ) async {
    final _Images images = _Images();
    final _Stage stage = await _showing(tester, images, 'a');
    stage.director.request(stage.push(_page()), 'b');
    images.ready('b');
    await _frame(tester);
    expect(stage.director.shown, 'b');

    stage.pop();
    await _frame(tester);

    expect(stage.director.shown, 'a');
  });

  testWidgets('a hidden page that asks waits until it is on screen again', (
    WidgetTester tester,
  ) async {
    final _Images images = _Images();
    final _Stage stage = _stage(images);
    final Route<dynamic> under = stage.push(_page());
    stage.director.request(stage.push(_page()), 'b');
    images.ready('b');
    await _frame(tester);

    stage.director.request(under, 'a');
    images.ready('a');
    await _frame(tester);
    expect(stage.director.shown, 'b');

    stage.pop();
    await _frame(tester);
    expect(stage.director.shown, 'a');
  });

  testWidgets('a page that has left asks in vain', (WidgetTester tester) async {
    final _Images images = _Images();
    final _Stage stage = await _showing(tester, images, 'a');
    final Route<dynamic> gone = stage.push(_page());
    stage.pop();
    await _frame(tester);

    stage.director.request(gone, 'b');
    images.ready('b');
    await _frame(tester);

    expect(stage.director.shown, 'a');
    expect(stage.told, 0);
  });

  testWidgets('a dialog is never the page on screen', (
    WidgetTester tester,
  ) async {
    final _Images images = _Images();
    final _Stage stage = await _showing(tester, images, 'a');
    final Route<dynamic> page = stage.top;

    stage.push(_dialog());
    await _frame(tester);
    expect(stage.director.shown, 'a');
    expect(stage.told, 0);

    stage.director.request(page, 'b');
    images.ready('b');
    await _frame(tester);
    expect(stage.director.shown, 'b');

    stage.pop();
    await _frame(tester);
    expect(stage.director.shown, 'b');
    expect(stage.told, 1);
  });

  testWidgets('an image ready after its page left the screen is dropped', (
    WidgetTester tester,
  ) async {
    final _Images images = _Images();
    final _Stage stage = _stage(images);
    stage.director.request(stage.push(_page()), 'a');
    stage.director.fallback(stage.push(_page()), null);
    await _frame(tester);

    images.ready('a');
    await _frame(tester);

    expect(stage.director.shown, isNull);
    expect(stage.told, 0);
  });

  testWidgets('an image that fails to load shows none', (
    WidgetTester tester,
  ) async {
    final _Images images = _Images();
    final _Stage stage = await _showing(tester, images, 'a');

    stage.director.request(stage.push(_page()), 'b');
    images.fail('b');
    await _frame(tester);

    expect(stage.director.shown, isNull);
  });

  testWidgets('a request outranks the fallback, whichever came first', (
    WidgetTester tester,
  ) async {
    final _Images images = _Images();
    final _Stage stage = _stage(images);
    final Route<dynamic> first = stage.push(_page());
    stage.director
      ..request(first, 'a')
      ..fallback(first, null);
    images.ready('a');
    await _frame(tester);
    expect(stage.director.shown, 'a');

    final Route<dynamic> second = stage.push(_page());
    stage.director
      ..fallback(second, null)
      ..request(second, 'b');
    images.ready('b');
    await _frame(tester);
    expect(stage.director.shown, 'b');
  });

  testWidgets('a replace or a removal shows the new page on top', (
    WidgetTester tester,
  ) async {
    final _Images images = _Images();
    final _Stage stage = await _showing(tester, images, 'a');
    final Route<dynamic> middle = stage.push(_page());
    stage.director.request(middle, 'b');
    images.ready('b');
    stage.director.request(stage.push(_page()), 'c');
    images.ready('c');
    await _frame(tester);
    expect(stage.director.shown, 'c');

    final Route<dynamic> next = _page();
    stage.replace(next);
    stage.director.fallback(next, null);
    await _frame(tester);
    expect(stage.director.shown, isNull);

    stage.remove(middle);
    stage.pop();
    await _frame(tester);
    expect(stage.director.shown, 'a');
  });

  testWidgets('a jump back over several pages shows only where it lands', (
    WidgetTester tester,
  ) async {
    final _Images images = _Images();
    final _Stage stage = await _showing(tester, images, 'a');
    stage.director.fallback(stage.push(_page()), null);
    stage.director.request(stage.push(_page()), 'c');
    images.ready('c');
    await _frame(tester);
    expect(stage.director.shown, 'c');
    stage.seen.clear();

    stage
      ..pop()
      ..pop();
    await _frame(tester);

    expect(stage.seen, <String?>[
      'a',
    ], reason: 'browser back two entries pops one page after the other');
  });

  testWidgets('a page pushed before the old ones are removed takes over', (
    WidgetTester tester,
  ) async {
    final _Images images = _Images();
    final _Stage stage = await _showing(tester, images, 'a');
    final Route<dynamic> first = stage.top;

    stage.director.request(stage.push(_page()), 'a');
    stage.remove(first);
    await _frame(tester);

    expect(
      stage.seen,
      isEmpty,
      reason: 'push and remove until reports the push, then each removal',
    );
    expect(stage.director.shown, 'a');
  });
}
