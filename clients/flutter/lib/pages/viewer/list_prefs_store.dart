import 'package:shared_preferences/shared_preferences.dart';

typedef ListOrdering = ({String sort, String order});

class ListPrefsStore {
  const ListPrefsStore(this.scope);

  final String scope;

  String get _key => 'shadowmask.sort.$scope';

  Future<ListOrdering?> load() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    final List<String> parts = (prefs.getString(_key) ?? '').split(':');
    if (parts.length != 2 || parts[0].isEmpty || parts[1].isEmpty) {
      return null;
    }
    return (sort: parts[0], order: parts[1]);
  }

  Future<void> save(String sort, String order) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setString(_key, '$sort:$order');
  }
}
