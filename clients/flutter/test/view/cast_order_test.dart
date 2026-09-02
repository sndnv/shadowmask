import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/view/cast_order.dart';

Credit _credit(String id, CreditRole role, int order) => Credit(
  person: PersonRef(id: id, name: 'Person $id'),
  role: role,
  order: order,
);

List<String> _ids(List<Credit> credits) =>
    billedActors(credits).map((PersonRef p) => p.id).toList();

void main() {
  test('no credits at all yields no actors', () {
    expect(billedActors(const <Credit>[]), isEmpty);
  });

  test('crew alone yields no actors', () {
    expect(
      _ids(<Credit>[
        _credit('d1', CreditRole.director, 0),
        _credit('w1', CreditRole.writer, 0),
      ]),
      isEmpty,
    );
  });

  test('actors come back in billing order, whatever order they arrive in', () {
    expect(
      _ids(<Credit>[
        _credit('a3', CreditRole.actor, 3),
        _credit('a1', CreditRole.actor, 1),
        _credit('a2', CreditRole.actor, 2),
      ]),
      <String>['a1', 'a2', 'a3'],
    );
  });

  test('crew given order zero never leads the billing', () {
    expect(
      _ids(<Credit>[
        _credit('d1', CreditRole.director, 0),
        _credit('a1', CreditRole.actor, 5),
      ]),
      <String>['a1'],
    );
  });

  test('a tie on order falls back to the id, so the order is stable', () {
    expect(
      _ids(<Credit>[
        _credit('zed', CreditRole.actor, 0),
        _credit('abe', CreditRole.actor, 0),
      ]),
      <String>['abe', 'zed'],
    );
    expect(
      _ids(<Credit>[
        _credit('abe', CreditRole.actor, 0),
        _credit('zed', CreditRole.actor, 0),
      ]),
      <String>['abe', 'zed'],
    );
  });

  test('a person credited twice appears once', () {
    expect(
      _ids(<Credit>[
        _credit('a1', CreditRole.actor, 0),
        _credit('a1', CreditRole.actor, 7),
        _credit('a2', CreditRole.actor, 1),
      ]),
      <String>['a1', 'a2'],
    );
  });
}
