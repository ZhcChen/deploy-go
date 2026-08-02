import 'package:deploy_go_admin/security/secure_cookie_storage.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/memory_secure_store.dart';

void main() {
  test('Cookie storage 只通过安全键值存储读写并可完整清除', () async {
    final secureStore = MemorySecureStore();
    final storage = SecureCookieStorage(secureStore);

    await storage.init(true, false);
    await storage.write('.index', '["api.example.test"]');
    await storage.write('api.example.test', 'encrypted-cookie-payload');

    expect(await storage.read('.index'), '["api.example.test"]');
    expect(
      secureStore.values.keys,
      containsAll(<String>[
        'deploy_go.cookie..index',
        'deploy_go.cookie.api.example.test',
      ]),
    );

    await storage.deleteAll(<String>['.index', 'api.example.test']);
    expect(secureStore.values, isEmpty);
  });
}
