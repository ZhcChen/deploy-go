import 'dart:async';

import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/providers.dart';
import '../shared/cursor_collection.dart';

final usersProvider =
    StateNotifierProvider.autoDispose<
      CursorCollectionController<UserResponse>,
      CursorCollectionState<UserResponse>
    >((ref) {
      final gateway = ref.watch(mobileDataGatewayProvider);
      final controller = CursorCollectionController<UserResponse>(
        (after) => gateway.users(after: after),
        (item) => item.id,
      );
      unawaited(controller.refresh());
      return controller;
    });

final userProvider = FutureProvider.autoDispose.family<UserResponse, String>(
  (ref, id) => ref.watch(mobileDataGatewayProvider).user(id),
);
