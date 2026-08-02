import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../resources/resource_providers.dart';
import '../shared/mobile_widgets.dart';

class OverviewPage extends ConsumerWidget {
  const OverviewPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final applications = ref.watch(applicationsProvider);
    final nodes = ref.watch(nodesProvider);
    final loading = applications.loading || nodes.loading;
    final error = applications.error ?? nodes.error;
    if (loading && applications.items.isEmpty && nodes.items.isEmpty) {
      return const MobilePageScaffold(
        title: '概览',
        child: Center(child: CircularProgressIndicator()),
      );
    }
    if (error != null && applications.items.isEmpty && nodes.items.isEmpty) {
      return MobilePageScaffold(
        title: '概览',
        child: MobileStateView.error(
          error,
          onRetry: () {
            ref.read(applicationsProvider.notifier).refresh();
            ref.read(nodesProvider.notifier).refresh();
          },
        ),
      );
    }
    final abnormalApplications = applications.items
        .where((item) => item.status == 'error')
        .length;
    final unhealthyNodes = nodes.items
        .where((item) => item.status != 'online')
        .length;
    return MobilePageScaffold(
      title: '概览',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          LayoutBuilder(
            builder: (context, constraints) {
              final width = (constraints.maxWidth - 10) / 2;
              final scaledBody = MediaQuery.textScalerOf(context).scale(14);
              final height =
                  124.0 + (scaledBody - 14).clamp(0, 28).toDouble() * 4;
              Widget tile({
                required String label,
                required String value,
                required IconData icon,
              }) => SizedBox(
                width: width,
                height: height,
                child: SummaryTile(label: label, value: value, icon: icon),
              );
              return Wrap(
                spacing: 10,
                runSpacing: 10,
                children: <Widget>[
                  tile(
                    label: '可访问应用',
                    value: '${applications.items.length}',
                    icon: Icons.apps_outlined,
                  ),
                  tile(
                    label: '关联节点',
                    value: '${nodes.items.length}',
                    icon: Icons.dns_outlined,
                  ),
                  tile(
                    label: '异常应用',
                    value: '$abnormalApplications',
                    icon: Icons.error_outline,
                  ),
                  tile(
                    label: '需关注节点',
                    value: '$unhealthyNodes',
                    icon: Icons.monitor_heart_outlined,
                  ),
                ],
              );
            },
          ),
          const SectionHeader('快捷入口'),
          ListTile(
            minTileHeight: 56,
            leading: const Icon(Icons.widgets_outlined),
            title: const Text('查看应用与节点'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => context.go('/resources'),
          ),
          ListTile(
            minTileHeight: 56,
            leading: const Icon(Icons.rocket_launch_outlined),
            title: const Text('查看部署记录'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => context.go('/deployments'),
          ),
        ],
      ),
    );
  }
}
