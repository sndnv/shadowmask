Future<String> fakeDigest(String username, String password) async =>
    'digest:$username:$password';
