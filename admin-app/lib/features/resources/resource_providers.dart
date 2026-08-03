import 'dart:async';

import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/contracts.dart';
import '../../app/providers.dart';
import '../shared/cursor_collection.dart';

final applicationsProvider =
    StateNotifierProvider.autoDispose<
      CursorCollectionController<ApplicationResponse>,
      CursorCollectionState<ApplicationResponse>
    >((ref) {
      final gateway = ref.watch(mobileDataGatewayProvider);
      final controller = CursorCollectionController<ApplicationResponse>(
        (after) => gateway.applications(after: after),
        (item) => item.id,
      );
      unawaited(controller.refresh());
      return controller;
    });

final nodesProvider =
    StateNotifierProvider.autoDispose<
      CursorCollectionController<NodeResponse>,
      CursorCollectionState<NodeResponse>
    >((ref) {
      final gateway = ref.watch(mobileDataGatewayProvider);
      final controller = CursorCollectionController<NodeResponse>(
        (after) => gateway.nodes(after: after),
        (item) => item.id,
      );
      unawaited(controller.refresh());
      return controller;
    });

final applicationProvider = FutureProvider.autoDispose
    .family<ApplicationResponse, String>(
      (ref, id) => ref.watch(mobileDataGatewayProvider).application(id),
    );

final nodeProvider = FutureProvider.autoDispose.family<NodeResponse, String>(
  (ref, id) => ref.watch(mobileDataGatewayProvider).node(id),
);

final nodeAgentProvider = FutureProvider.autoDispose
    .family<AgentStatusView?, String>((ref, nodeId) async {
      final identity = ref.watch(
        sessionControllerProvider.select(
          (state) => state.session?.user.identity,
        ),
      );
      if (identity != 'administrator') return null;
      final agent = await ref
          .watch(mobileDataGatewayProvider)
          .agentForNode(nodeId);
      if (agent == null) {
        return const AgentStatusView(
          status: 'offline',
          versionState: AgentVersionState.unknown,
        );
      }
      final version = agent.agentVersion;
      return AgentStatusView(
        status: agent.status == 'online' ? 'online' : 'offline',
        name: agent.name,
        version: version,
        versionState: version == null
            ? AgentVersionState.unknown
            : version == supportedAgentVersion
            ? AgentVersionState.current
            : AgentVersionState.mismatch,
        hostname: agent.hostname,
        architecture: agent.architecture,
        lastSeenAt: agent.lastSeenAt,
      );
    });
