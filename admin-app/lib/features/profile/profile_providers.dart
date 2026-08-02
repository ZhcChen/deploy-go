import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/providers.dart';

final profileProvider = FutureProvider.autoDispose<UserIdentity>(
  (ref) => ref.watch(mobileDataGatewayProvider).profile(),
);

final preferencesProvider = FutureProvider.autoDispose<UserPreferencesResponse>(
  (ref) => ref.watch(mobileDataGatewayProvider).preferences(),
);
