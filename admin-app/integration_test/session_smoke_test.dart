import 'package:deploy_go_admin/security/secure_key_value_store.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('平台安全存储可跨实例恢复并清除会话值', (tester) async {
    const key = 'deploy_go.integration.session';
    final first = FlutterSecureKeyValueStore();
    await first.delete(key);
    await first.write(key, 'fixture-session-value');

    final restored = FlutterSecureKeyValueStore();
    expect(await restored.read(key), 'fixture-session-value');

    await restored.delete(key);
    expect(await first.read(key), isNull);
  });
}
