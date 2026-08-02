import 'secure_key_value_store.dart';

class SecureSessionStore {
  SecureSessionStore(this._store);

  static const _csrfKey = 'deploy_go.session.csrf';
  final SecureKeyValueStore _store;

  Future<String?> readCsrfToken() => _store.read(_csrfKey);

  Future<void> writeCsrfToken(String token) => _store.write(_csrfKey, token);

  Future<void> clearCsrfToken() => _store.delete(_csrfKey);
}
