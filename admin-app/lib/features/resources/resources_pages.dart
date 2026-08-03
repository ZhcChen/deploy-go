import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../api/contracts.dart';
import '../../api/mobile_data_gateway.dart';
import '../shared/cursor_collection.dart';
import '../shared/mobile_widgets.dart';
import 'resource_providers.dart';

enum ResourceSegment { applications, nodes }

class ResourcesPage extends ConsumerStatefulWidget {
  const ResourcesPage({super.key});

  @override
  ConsumerState<ResourcesPage> createState() => _ResourcesPageState();
}

class _ResourcesPageState extends ConsumerState<ResourcesPage> {
  ResourceSegment segment = ResourceSegment.applications;

  @override
  Widget build(BuildContext context) {
    final applications = ref.watch(applicationsProvider);
    final nodes = ref.watch(nodesProvider);
    return MobilePageScaffold(
      title: '资源',
      scrollable: false,
      child: Column(
        children: <Widget>[
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
            child: SizedBox(
              width: double.infinity,
              child: SegmentedButton<ResourceSegment>(
                segments: const <ButtonSegment<ResourceSegment>>[
                  ButtonSegment(
                    value: ResourceSegment.applications,
                    icon: Icon(Icons.apps_outlined),
                    label: Text('应用'),
                  ),
                  ButtonSegment(
                    value: ResourceSegment.nodes,
                    icon: Icon(Icons.dns_outlined),
                    label: Text('节点'),
                  ),
                ],
                selected: <ResourceSegment>{segment},
                onSelectionChanged: (value) {
                  setState(() => segment = value.first);
                },
              ),
            ),
          ),
          Expanded(
            child: segment == ResourceSegment.applications
                ? _ApplicationList(state: applications)
                : _NodeList(state: nodes),
          ),
        ],
      ),
    );
  }
}

class _ApplicationList extends ConsumerWidget {
  const _ApplicationList({required this.state});
  final CursorCollectionState<ApplicationResponse> state;

  @override
  Widget build(BuildContext context, WidgetRef ref) => _CollectionView(
    state: state,
    emptyTitle: '还没有可访问的应用',
    onRefresh: ref.read(applicationsProvider.notifier).refresh,
    onLoadMore: ref.read(applicationsProvider.notifier).loadMore,
    itemBuilder: (context, index) {
      final item = state.items[index];
      return MobileResourceRow(
        icon: Icons.inventory_2_outlined,
        title: item.name,
        subtitle: item.description.isEmpty ? item.slug : item.description,
        status: _applicationStatus(item.status),
        onTap: () => context.go('/resources/applications/${item.id}'),
      );
    },
  );
}

class _NodeList extends ConsumerWidget {
  const _NodeList({required this.state});
  final CursorCollectionState<NodeResponse> state;

  @override
  Widget build(BuildContext context, WidgetRef ref) => _CollectionView(
    state: state,
    emptyTitle: '还没有可访问的节点',
    onRefresh: ref.read(nodesProvider.notifier).refresh,
    onLoadMore: ref.read(nodesProvider.notifier).loadMore,
    itemBuilder: (context, index) {
      final item = state.items[index];
      return MobileResourceRow(
        icon: Icons.dns_outlined,
        title: item.name,
        subtitle: item.workRoot ?? '等待 Agent 上报工作目录',
        status: nodeStatus(item.status),
        onTap: () => context.go('/resources/nodes/${item.id}'),
      );
    },
  );
}

class _CollectionView<T> extends StatelessWidget {
  const _CollectionView({
    required this.state,
    required this.emptyTitle,
    required this.onRefresh,
    required this.onLoadMore,
    required this.itemBuilder,
  });

  final CursorCollectionState<T> state;
  final String emptyTitle;
  final Future<void> Function() onRefresh;
  final Future<void> Function() onLoadMore;
  final IndexedWidgetBuilder itemBuilder;

  @override
  Widget build(BuildContext context) {
    if (state.loading && state.items.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    if (state.error != null && state.items.isEmpty) {
      return MobileStateView.error(state.error!, onRetry: () => onRefresh());
    }
    if (state.items.isEmpty) {
      return RefreshIndicator(
        onRefresh: onRefresh,
        child: ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          children: <Widget>[MobileStateView(title: emptyTitle)],
        ),
      );
    }
    return RefreshIndicator(
      onRefresh: onRefresh,
      child: ListView.separated(
        padding: const EdgeInsets.fromLTRB(16, 4, 16, 24),
        itemCount:
            state.items.length +
            (state.error != null ? 1 : 0) +
            (state.hasMore ? 1 : 0),
        separatorBuilder: (_, _) => const Divider(height: 1),
        itemBuilder: (context, index) {
          if (state.error != null && index == 0) {
            return _InlineError(
              error: state.error!,
              onRetry: state.errorFromRefresh ? onRefresh : onLoadMore,
            );
          }
          final itemIndex = index - (state.error != null ? 1 : 0);
          if (itemIndex < state.items.length) {
            return itemBuilder(context, itemIndex);
          }
          return Padding(
            padding: const EdgeInsets.symmetric(vertical: 16),
            child: Center(
              child: OutlinedButton.icon(
                onPressed: state.loadingMore ? null : onLoadMore,
                icon: state.loadingMore
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.expand_more),
                label: Text(state.loadingMore ? '正在加载' : '加载更多'),
              ),
            ),
          );
        },
      ),
    );
  }
}

class _InlineError extends StatelessWidget {
  const _InlineError({required this.error, required this.onRetry});
  final Object error;
  final Future<void> Function() onRetry;

  @override
  Widget build(BuildContext context) {
    final failure = error is ApiFailureException
        ? (error as ApiFailureException).failure
        : null;
    return ListTile(
      minTileHeight: 56,
      contentPadding: EdgeInsets.zero,
      leading: const Icon(Icons.error_outline),
      title: Text(failure?.message ?? '部分数据加载失败'),
      subtitle: failure?.requestId.isNotEmpty == true
          ? Text('Request ID: ${failure!.requestId}')
          : null,
      trailing: IconButton(
        tooltip: '重试',
        onPressed: () => onRetry(),
        icon: const Icon(Icons.refresh),
      ),
    );
  }
}

class ApplicationDetailPage extends ConsumerWidget {
  const ApplicationDetailPage({required this.id, super.key});
  final String id;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final value = ref.watch(applicationProvider(id));
    return MobilePageScaffold(
      title: '应用详情',
      child: value.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, _) => MobileStateView.error(
          error,
          onRetry: () => ref.invalidate(applicationProvider(id)),
        ),
        data: (application) => Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            _DetailTitle(
              icon: Icons.inventory_2_outlined,
              title: application.name,
              subtitle: application.slug,
              badge: _applicationStatus(application.status),
            ),
            const SectionHeader('应用信息'),
            _DetailLine(label: '说明', value: application.description),
            _DetailLine(label: '最近更新', value: application.updatedAt),
            const SizedBox(height: 20),
            FilledButton.icon(
              onPressed: () => context.go('/deployments'),
              icon: const Icon(Icons.rocket_launch_outlined),
              label: const Text('查看部署'),
            ),
          ],
        ),
      ),
    );
  }
}

class NodeDetailPage extends ConsumerStatefulWidget {
  const NodeDetailPage({required this.id, super.key});
  final String id;

  @override
  ConsumerState<NodeDetailPage> createState() => _NodeDetailPageState();
}

class _NodeDetailPageState extends ConsumerState<NodeDetailPage>
    with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) _refresh();
  }

  void _refresh() {
    ref.invalidate(nodeProvider(widget.id));
    ref.invalidate(nodeAgentProvider(widget.id));
  }

  @override
  Widget build(BuildContext context) {
    final value = ref.watch(nodeProvider(widget.id));
    final agent = ref.watch(nodeAgentProvider(widget.id));
    return MobilePageScaffold(
      title: '节点详情',
      child: value.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, _) => MobileStateView.error(error, onRetry: _refresh),
        data: (node) => Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            _DetailTitle(
              icon: Icons.dns_outlined,
              title: node.name,
              subtitle: node.id,
              badge: nodeStatus(node.status),
            ),
            const SectionHeader('Agent 状态'),
            _DetailLine(
              label: '连接',
              value: node.status == 'online' ? '在线' : '离线',
            ),
            _DetailLine(label: '工作目录', value: node.workRoot ?? '尚未上报'),
            _DetailLine(label: '最近检查', value: node.checkedAt ?? '尚未检查'),
            agent.when(
              loading: () => const Padding(
                padding: EdgeInsets.symmetric(vertical: 20),
                child: Center(child: CircularProgressIndicator()),
              ),
              error: (error, _) => _AgentDiagnosticError(
                error: error,
                onRetry: () => ref.invalidate(nodeAgentProvider(widget.id)),
              ),
              data: (status) => status == null
                  ? const SizedBox.shrink()
                  : _AgentDiagnostics(status: status),
            ),
          ],
        ),
      ),
    );
  }
}

class _AgentDiagnostics extends StatelessWidget {
  const _AgentDiagnostics({required this.status});
  final AgentStatusView status;

  @override
  Widget build(BuildContext context) {
    final versionLabel = switch (status.versionState) {
      AgentVersionState.current => status.version!,
      AgentVersionState.mismatch => '${status.version} · 版本异常',
      AgentVersionState.unknown => '尚未上报',
    };
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        const SectionHeader('运行诊断'),
        _DetailLine(label: 'Agent', value: status.name ?? '尚未关联'),
        _DetailLine(label: '版本', value: versionLabel),
        _DetailLine(label: '主机', value: status.hostname ?? '尚未上报'),
        _DetailLine(label: '架构', value: status.architecture ?? '尚未上报'),
        _DetailLine(label: '最后在线', value: status.lastSeenAt ?? '从未连接'),
        const SizedBox(height: 12),
        Text(
          'Agent 安装、撤销和升级请在 Web 管理端完成。',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}

class _AgentDiagnosticError extends StatelessWidget {
  const _AgentDiagnosticError({required this.error, required this.onRetry});
  final Object error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final failure = error is ApiFailureException
        ? (error as ApiFailureException).failure
        : null;
    return Padding(
      padding: const EdgeInsets.only(top: 16),
      child: Material(
        color: Theme.of(context).colorScheme.surfaceContainerLow,
        borderRadius: BorderRadius.circular(8),
        child: ListTile(
          leading: const Icon(Icons.info_outline),
          title: Text(failure?.message ?? 'Agent 诊断暂不可用'),
          subtitle: failure?.requestId.isNotEmpty == true
              ? Text('Request ID: ${failure!.requestId}')
              : null,
          trailing: IconButton(
            tooltip: '重试 Agent 诊断',
            onPressed: onRetry,
            icon: const Icon(Icons.refresh),
          ),
        ),
      ),
    );
  }
}

class _DetailTitle extends StatelessWidget {
  const _DetailTitle({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.badge,
  });
  final IconData icon;
  final String title;
  final String subtitle;
  final Widget badge;

  @override
  Widget build(BuildContext context) => Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: <Widget>[
      CircleAvatar(radius: 25, child: Icon(icon)),
      const SizedBox(width: 14),
      Expanded(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(title, style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 4),
            Text(subtitle),
          ],
        ),
      ),
      const SizedBox(width: 8),
      badge,
    ],
  );
}

class _DetailLine extends StatelessWidget {
  const _DetailLine({required this.label, required this.value});
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 9),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        SizedBox(width: 92, child: Text(label)),
        Expanded(child: Text(value.isEmpty ? '-' : value)),
      ],
    ),
  );
}

StatusBadge _applicationStatus(String status) => switch (status) {
  'active' => const StatusBadge(label: '正常', kind: StatusKind.success),
  'deploying' => const StatusBadge(label: '部署中', kind: StatusKind.warning),
  'error' => const StatusBadge(label: '异常', kind: StatusKind.danger),
  'archived' => const StatusBadge(label: '已归档', kind: StatusKind.neutral),
  _ => StatusBadge(label: status, kind: StatusKind.neutral),
};

StatusBadge nodeStatus(String status) => switch (status) {
  'online' => const StatusBadge(label: '在线', kind: StatusKind.success),
  _ => const StatusBadge(label: '离线', kind: StatusKind.neutral),
};
