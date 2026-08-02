import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'api/api_environment.dart';
import 'api/auth_repository.dart';
import 'api/deploy_go_api.dart';
import 'api/mobile_data_gateway.dart';
import 'app/deploy_go_app.dart';
import 'app/providers.dart';
import 'security/secure_key_value_store.dart';
import 'security/secure_session_store.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final secureStore = FlutterSecureKeyValueStore();
  final api = await DeployGoApi.create(
    environment: ApiEnvironment.fromBuildConfiguration(),
    secureStore: secureStore,
  );
  final auth = AuthRepository(api, SecureSessionStore(secureStore));
  final mobileData = DeployGoMobileDataGateway(
    api,
    SecureSessionStore(secureStore),
  );
  runApp(
    ProviderScope(
      overrides: <Override>[
        authGatewayProvider.overrideWithValue(auth),
        mobileDataGatewayProvider.overrideWithValue(mobileData),
      ],
      child: const DeployGoApp(),
    ),
  );
}
