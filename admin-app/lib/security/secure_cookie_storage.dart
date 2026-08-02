import 'package:cookie_jar/cookie_jar.dart';

import 'secure_key_value_store.dart';

class SecureCookieStorage implements Storage {
  SecureCookieStorage(this._store);

  static const _prefix = 'deploy_go.cookie.';
  final SecureKeyValueStore _store;

  @override
  Future<void> init(bool persistSession, bool ignoreExpires) async {}

  @override
  Future<String?> read(String key) => _store.read('$_prefix$key');

  @override
  Future<void> write(String key, String value) =>
      _store.write('$_prefix$key', value);

  @override
  Future<void> delete(String key) => _store.delete('$_prefix$key');

  @override
  Future<void> deleteAll(List<String> keys) async {
    for (final key in keys) {
      await delete(key);
    }
  }
}
