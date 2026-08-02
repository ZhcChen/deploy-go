import 'dart:math';

import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../api/sse_client.dart';
import '../../app/providers.dart';
import '../resources/resource_providers.dart';
import '../shared/cursor_collection.dart';
import '../shared/mobile_widgets.dart';
import 'deployment_providers.dart';

class DeploymentsPage extends ConsumerWidget {
  const DeploymentsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(deploymentsProvider);
    return MobilePageScaffold(
      key: const ValueKey<String>('deployment-root'),
      title: '部署',
      scrollable: false,
      actions: <Widget>[
        IconButton(
          tooltip: '发起部署',
          onPressed: () => context.go('/deployments/new'),
          icon: const Icon(Icons.rocket_launch_outlined),
        ),
      ],
      child: _DeploymentCollection(state: state),
    );
  }
}

class _DeploymentCollection extends ConsumerWidget {
  const _DeploymentCollection({required this.state});

  final CursorCollectionState<DeploymentResponse> state;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = ref.read(deploymentsProvider.notifier);
    if (state.loading && state.items.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    if (state.error != null && state.items.isEmpty) {
      return MobileStateView.error(state.error!, onRetry: controller.refresh);
    }
    if (state.items.isEmpty) {
      return RefreshIndicator(
        onRefresh: controller.refresh,
        child: ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          children: <Widget>[
            MobileStateView(
              title: '还没有部署记录',
              icon: Icons.rocket_launch_outlined,
              onRetry: () => context.go('/deployments/new'),
            ),
          ],
        ),
      );
    }
    return RefreshIndicator(
      onRefresh: controller.refresh,
      child: ListView.separated(
        padding: const EdgeInsets.fromLTRB(16, 4, 16, 24),
        itemCount:
            state.items.length +
            (state.error != null ? 1 : 0) +
            (state.hasMore ? 1 : 0),
        separatorBuilder: (_, _) => const Divider(height: 1),
        itemBuilder: (context, index) {
          if (state.error != null && index == 0) {
            return ListTile(
              minTileHeight: 56,
              leading: const Icon(Icons.error_outline),
              title: const Text('部分部署记录加载失败'),
              trailing: IconButton(
                tooltip: '重试',
                onPressed: state.errorFromRefresh
                    ? controller.refresh
                    : controller.loadMore,
                icon: const Icon(Icons.refresh),
              ),
            );
          }
          final itemIndex = index - (state.error != null ? 1 : 0);
          if (itemIndex == state.items.length) {
            return Padding(
              padding: const EdgeInsets.symmetric(vertical: 16),
              child: Center(
                child: OutlinedButton.icon(
                  onPressed: state.loadingMore ? null : controller.loadMore,
                  icon: const Icon(Icons.expand_more),
                  label: Text(state.loadingMore ? '正在加载' : '加载更多'),
                ),
              ),
            );
          }
          final deployment = state.items[itemIndex];
          return MobileResourceRow(
            icon: Icons.rocket_launch_outlined,
            title: deployment.id,
            subtitle:
                '${deployment.phase} · ${_formatTime(deployment.createdAt)}',
            status: _deploymentStatus(deployment.status),
            onTap: () => context.go('/deployments/${deployment.id}'),
          );
        },
      ),
    );
  }
}

class NewDeploymentPage extends ConsumerStatefulWidget {
  const NewDeploymentPage({this.initialApplicationId, super.key});

  final String? initialApplicationId;

  @override
  ConsumerState<NewDeploymentPage> createState() => _NewDeploymentPageState();
}

class _NewDeploymentPageState extends ConsumerState<NewDeploymentPage> {
  final formKey = GlobalKey<FormState>();
  String? applicationId;
  String? targetId;
  List<DeploymentTargetResponse> targets = const [];
  Map<String, Object?> parameters = <String, Object?>{};
  DeploymentPreviewResponse? preview;
  bool loadingTargets = false;
  bool previewing = false;
  bool confirming = false;
  bool dirty = false;
  Object? error;
  String? failedOperation;
  String? idempotencyKey;

  bool get busy => loadingTargets || previewing || confirming;

  @override
  void initState() {
    super.initState();
    applicationId = widget.initialApplicationId;
    if (applicationId != null) _loadTargets(applicationId!);
  }

  @override
  Widget build(BuildContext context) {
    final applications = ref
        .watch(applicationsProvider)
        .items
        .where((item) => item.status == 'active')
        .toList(growable: false);
    final selectedTarget = targets
        .where((item) => item.id == targetId)
        .firstOrNull;
    return PopScope(
      canPop: !dirty && !busy,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop && !busy) _confirmDiscard();
      },
      child: MobilePageScaffold(
        title: '发起部署',
        child: Form(
          key: formKey,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              const SectionHeader('1. 选择目标'),
              DropdownButtonFormField<String>(
                key: const ValueKey<String>('deployment-application'),
                initialValue: applicationId,
                decoration: const InputDecoration(labelText: '应用'),
                items: applications
                    .map(
                      (item) => DropdownMenuItem(
                        value: item.id,
                        child: Text(item.name),
                      ),
                    )
                    .toList(growable: false),
                onChanged: busy
                    ? null
                    : (value) {
                        setState(() {
                          applicationId = value;
                          targetId = null;
                          targets = const [];
                          dirty = true;
                          _invalidatePreview();
                        });
                        if (value != null) _loadTargets(value);
                      },
                validator: (value) => value == null ? '请选择应用' : null,
              ),
              const SizedBox(height: 14),
              DropdownButtonFormField<String>(
                key: const ValueKey<String>('deployment-target'),
                initialValue: targetId,
                decoration: const InputDecoration(labelText: '部署目标'),
                items: targets
                    .where((item) => item.status == 'active')
                    .map(
                      (item) => DropdownMenuItem(
                        value: item.id,
                        child: Text('${item.environment} · ${item.scriptPath}'),
                      ),
                    )
                    .toList(growable: false),
                onChanged: busy
                    ? null
                    : (value) => setState(() {
                        targetId = value;
                        parameters = value == null
                            ? <String, Object?>{}
                            : _schemaDefaults(
                                targets
                                    .firstWhere((item) => item.id == value)
                                    .parameterSchema
                                    ?.value,
                              );
                        dirty = true;
                        _invalidatePreview();
                      }),
                validator: (value) => value == null ? '请选择部署目标' : null,
              ),
              if (loadingTargets)
                const Padding(
                  padding: EdgeInsets.only(top: 14),
                  child: LinearProgressIndicator(),
                ),
              const SectionHeader('2. 填写受控参数'),
              if (selectedTarget == null)
                const Text('请先选择可用的部署目标。')
              else
                _ParameterEditor(
                  schema: selectedTarget.parameterSchema?.value,
                  value: parameters,
                  enabled: !busy,
                  onChanged: (value) => setState(() {
                    parameters = value;
                    dirty = true;
                    _invalidatePreview();
                  }),
                ),
              if (error != null) ...<Widget>[
                const SizedBox(height: 14),
                MobileStateView.error(error!, onRetry: _retryPageError),
              ],
              const SizedBox(height: 20),
              FilledButton.icon(
                key: const ValueKey<String>('preview-deployment-button'),
                onPressed: selectedTarget == null || busy ? null : _preview,
                icon: const Icon(Icons.preview_outlined),
                label: Text(previewing ? '正在生成预览' : '生成部署预览'),
              ),
              if (preview != null) ...<Widget>[
                const SectionHeader('3. 核对并确认'),
                _InfoLine(label: '应用', value: preview!.applicationName),
                _InfoLine(label: '节点', value: preview!.nodeName),
                _InfoLine(label: '环境', value: preview!.environment),
                _InfoLine(label: '脚本', value: preview!.scriptPath),
                _InfoLine(label: 'Snapshot', value: preview!.snapshotHash),
                const SizedBox(height: 18),
                FilledButton.icon(
                  key: const ValueKey<String>('confirm-deployment-button'),
                  onPressed: confirming ? null : _confirm,
                  icon: const Icon(Icons.rocket_launch_outlined),
                  label: Text(confirming ? '正在确认' : '确认并发起部署'),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }

  Future<void> _loadTargets(String id) async {
    setState(() {
      loadingTargets = true;
      error = null;
      failedOperation = null;
    });
    try {
      final loaded = await ref
          .read(mobileDataGatewayProvider)
          .deploymentTargets(id);
      if (!mounted || applicationId != id) return;
      setState(() => targets = loaded);
    } catch (caught) {
      if (mounted) {
        setState(() {
          error = caught;
          failedOperation = 'targets';
        });
      }
    } finally {
      if (mounted && applicationId == id) {
        setState(() => loadingTargets = false);
      }
    }
  }

  Future<void> _preview() async {
    final id = targetId;
    if (id == null || previewing || confirming) return;
    if (formKey.currentState?.validate() != true) return;
    setState(() {
      previewing = true;
      error = null;
      failedOperation = null;
    });
    try {
      final value = await ref
          .read(mobileDataGatewayProvider)
          .previewDeployment(id, parameters);
      if (mounted) {
        setState(() {
          preview = value;
          idempotencyKey = _newIdempotencyKey('deploy');
          dirty = true;
        });
      }
    } catch (caught) {
      if (mounted) {
        setState(() {
          error = caught;
          failedOperation = 'preview';
        });
      }
    } finally {
      if (mounted) setState(() => previewing = false);
    }
  }

  Future<void> _confirm() async {
    final value = preview;
    final key = idempotencyKey;
    if (value == null || key == null || confirming) return;
    setState(() {
      confirming = true;
      error = null;
      failedOperation = null;
    });
    try {
      final deployment = await ref
          .read(mobileDataGatewayProvider)
          .confirmDeployment(
            preview: value,
            parameters: parameters,
            idempotencyKey: key,
          );
      ref.invalidate(deploymentsProvider);
      dirty = false;
      if (mounted) context.go('/deployments/${deployment.id}');
    } catch (caught) {
      if (mounted) {
        setState(() {
          error = caught;
          failedOperation = 'confirm';
        });
      }
    } finally {
      if (mounted) setState(() => confirming = false);
    }
  }

  void _invalidatePreview() {
    preview = null;
    idempotencyKey = null;
  }

  void _retryPageError() {
    if (failedOperation == 'confirm') {
      _confirm();
    } else if (failedOperation == 'targets' && applicationId != null) {
      _loadTargets(applicationId!);
    } else {
      _preview();
    }
  }

  Future<void> _confirmDiscard() async {
    final discard = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('丢弃未提交的部署配置？'),
        content: const Text('当前参数和部署预览将不会保留。'),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('继续编辑'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('丢弃'),
          ),
        ],
      ),
    );
    if (discard == true && mounted) {
      setState(() => dirty = false);
      context.pop();
    }
  }
}

class DeploymentDetailPage extends ConsumerStatefulWidget {
  const DeploymentDetailPage({required this.id, super.key});

  final String id;

  @override
  ConsumerState<DeploymentDetailPage> createState() =>
      _DeploymentDetailPageState();
}

class _DeploymentDetailPageState extends ConsumerState<DeploymentDetailPage>
    with WidgetsBindingObserver {
  late final String retryKey = _newIdempotencyKey('retry');
  final logScroll = ScrollController();
  bool following = true;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    logScroll.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    final controller = ref.read(deploymentDetailProvider(widget.id).notifier);
    if (state == AppLifecycleState.resumed) {
      _resume(controller);
    } else if (state == AppLifecycleState.paused ||
        state == AppLifecycleState.inactive ||
        state == AppLifecycleState.detached ||
        state == AppLifecycleState.hidden) {
      controller.enterBackground();
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(deploymentDetailProvider(widget.id));
    final controller = ref.read(deploymentDetailProvider(widget.id).notifier);
    final deployment = state.deployment;
    ref.listen(deploymentDetailProvider(widget.id), (previous, next) {
      if (following && previous?.logs.length != next.logs.length) {
        WidgetsBinding.instance.addPostFrameCallback((_) => _jumpToLogEnd());
      }
    });
    return MobilePageScaffold(
      title: '部署详情',
      child: state.loading && deployment == null
          ? const Center(child: CircularProgressIndicator())
          : state.error != null && deployment == null
          ? MobileStateView.error(state.error!, onRetry: controller.initialize)
          : deployment == null
          ? const MobileStateView(title: '部署不存在')
          : Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Row(
                  children: <Widget>[
                    Expanded(
                      child: Text(
                        deployment.id,
                        style: Theme.of(context).textTheme.titleLarge,
                      ),
                    ),
                    _deploymentStatus(deployment.status),
                  ],
                ),
                const SectionHeader('执行信息'),
                _InfoLine(label: '阶段', value: deployment.phase),
                _InfoLine(label: 'Snapshot', value: deployment.snapshotHash),
                _InfoLine(label: '结果', value: deployment.resultSummary ?? '-'),
                if (deployment.status == 'interrupted')
                  const Padding(
                    padding: EdgeInsets.only(top: 12),
                    child: Text('平台无法证明远端脚本的最终状态。请核对节点后再重试。'),
                  ),
                if (state.actionError != null) ...<Widget>[
                  const SizedBox(height: 12),
                  MobileStateView.error(
                    state.actionError!,
                    onRetry: () => _retryError(controller, state.failedAction),
                  ),
                ],
                if (deployment.status == 'queued' ||
                    deployment.status == 'running') ...<Widget>[
                  const SizedBox(height: 18),
                  OutlinedButton.icon(
                    key: const ValueKey<String>('cancel-deployment-button'),
                    onPressed: state.action == null
                        ? () => _confirmCancel(controller)
                        : null,
                    icon: const Icon(Icons.stop_circle_outlined),
                    label: Text(state.action == 'cancel' ? '正在取消' : '取消部署'),
                    style: OutlinedButton.styleFrom(
                      foregroundColor: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ],
                if (const <String>{
                  'failed',
                  'canceled',
                  'interrupted',
                }.contains(deployment.status)) ...<Widget>[
                  const SizedBox(height: 18),
                  FilledButton.icon(
                    key: const ValueKey<String>('retry-deployment-button'),
                    onPressed: state.action == null
                        ? () => _retry(controller)
                        : null,
                    icon: const Icon(Icons.replay_outlined),
                    label: Text(state.action == 'retry' ? '正在创建' : '重试部署'),
                  ),
                ],
                const SectionHeader('执行日志'),
                Wrap(
                  alignment: WrapAlignment.spaceBetween,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  runSpacing: 4,
                  children: <Widget>[
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Icon(_connectionIcon(state.connection), size: 18),
                        const SizedBox(width: 8),
                        Text(_connectionLabel(state.connection)),
                      ],
                    ),
                    Wrap(
                      crossAxisAlignment: WrapCrossAlignment.center,
                      children: <Widget>[
                        TextButton.icon(
                          onPressed: () =>
                              setState(() => following = !following),
                          icon: Icon(
                            following
                                ? Icons.pause_outlined
                                : Icons.play_arrow_outlined,
                          ),
                          label: Text(following ? '暂停跟随' : '恢复跟随'),
                        ),
                        IconButton(
                          tooltip: '跳到日志末尾',
                          onPressed: _jumpToLogEnd,
                          icon: const Icon(Icons.vertical_align_bottom),
                        ),
                        if (state.connection == SseConnectionState.ended &&
                            !isTerminalDeployment(deployment.status))
                          IconButton(
                            tooltip: '重新连接日志',
                            onPressed: controller.reconnect,
                            icon: const Icon(Icons.refresh),
                          ),
                      ],
                    ),
                  ],
                ),
                SizedBox(
                  height: 260,
                  child: Container(
                    key: const ValueKey<String>('deployment-log-view'),
                    margin: const EdgeInsets.only(top: 10),
                    padding: const EdgeInsets.all(12),
                    color: const Color(0xff111111),
                    child: SingleChildScrollView(
                      controller: logScroll,
                      child: SelectableText(
                        state.logs.isEmpty
                            ? '等待脚本输出...'
                            : state.logs
                                  .map(
                                    (log) =>
                                        '${log.sequence.toString().padLeft(4)} ${log.stream.padRight(6)} ${log.content}${log.truncated ? " [已截断]" : ""}',
                                  )
                                  .join('\n'),
                        style: const TextStyle(
                          color: Color(0xfff5f5f5),
                          fontFamily: 'monospace',
                          fontSize: 12,
                        ),
                      ),
                    ),
                  ),
                ),
                if (state.logs.length >= 1000)
                  const Padding(
                    padding: EdgeInsets.only(top: 8),
                    child: Text('为控制内存，仅显示最近 1000 条日志。'),
                  ),
              ],
            ),
    );
  }

  Future<void> _confirmCancel(DeploymentDetailController controller) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('取消这次部署？'),
        content: const Text('取消只会停止脚本，不会自动回滚应用变更。'),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('继续执行'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
            ),
            child: const Text('确认取消'),
          ),
        ],
      ),
    );
    if (confirmed == true) await controller.cancel();
  }

  Future<void> _resume(DeploymentDetailController controller) async {
    final authenticated = await ref
        .read(sessionControllerProvider.notifier)
        .refreshAuthenticatedSession();
    if (authenticated && mounted) await controller.enterForeground();
  }

  void _jumpToLogEnd() {
    if (!logScroll.hasClients) return;
    logScroll.animateTo(
      logScroll.position.maxScrollExtent,
      duration: const Duration(milliseconds: 160),
      curve: Curves.easeOut,
    );
  }

  Future<void> _retry(DeploymentDetailController controller) async {
    final saved = await controller.retry(retryKey);
    if (saved != null && mounted) {
      ref.invalidate(deploymentsProvider);
      context.go('/deployments/${saved.id}');
    }
  }

  void _retryError(
    DeploymentDetailController controller,
    String? failedAction,
  ) {
    if (failedAction == 'cancel') {
      controller.cancel();
    } else if (failedAction == 'retry') {
      _retry(controller);
    } else {
      controller.reconnect();
    }
  }
}

class _ParameterEditor extends StatelessWidget {
  const _ParameterEditor({
    required this.schema,
    required this.value,
    required this.enabled,
    required this.onChanged,
  });

  final Object? schema;
  final Map<String, Object?> value;
  final bool enabled;
  final ValueChanged<Map<String, Object?>> onChanged;

  @override
  Widget build(BuildContext context) {
    final parsed = _asMap(schema);
    final properties = _asMap(parsed['properties']);
    final requiredNames =
        (parsed['required'] as List?)?.whereType<String>().toSet() ??
        const <String>{};
    if (properties.isEmpty) return const Text('该目标不需要额外参数。');
    return Column(
      children: properties.entries
          .map((entry) {
            final name = entry.key;
            final property = _asMap(entry.value);
            final label = property['title']?.toString() ?? name;
            final type = property['type']?.toString();
            final options = (property['enum'] as List?)?.toList();
            if (type == 'boolean') {
              return SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: Text(label),
                subtitle: property['description'] == null
                    ? null
                    : Text(property['description'].toString()),
                value: value[name] == true,
                onChanged: enabled
                    ? (selected) =>
                          onChanged(<String, Object?>{...value, name: selected})
                    : null,
              );
            }
            if (options != null) {
              return Padding(
                padding: const EdgeInsets.only(bottom: 14),
                child: DropdownButtonFormField<Object>(
                  initialValue: value[name],
                  decoration: InputDecoration(labelText: label),
                  items: options
                      .map(
                        (option) => DropdownMenuItem(
                          value: option,
                          child: Text('$option'),
                        ),
                      )
                      .toList(growable: false),
                  onChanged: enabled
                      ? (selected) => onChanged(<String, Object?>{
                          ...value,
                          name: selected,
                        })
                      : null,
                  validator: (selected) =>
                      requiredNames.contains(name) && selected == null
                      ? '此参数为必填项'
                      : null,
                ),
              );
            }
            final numeric = type == 'integer' || type == 'number';
            return Padding(
              padding: const EdgeInsets.only(bottom: 14),
              child: TextFormField(
                key: ValueKey<String>('deployment-parameter-$name'),
                initialValue: value[name]?.toString() ?? '',
                enabled: enabled,
                keyboardType: numeric
                    ? TextInputType.number
                    : TextInputType.text,
                decoration: InputDecoration(
                  labelText: requiredNames.contains(name) ? '$label *' : label,
                  helperText: property['description']?.toString(),
                ),
                onChanged: (text) {
                  final next = <String, Object?>{...value};
                  if (text.isEmpty) {
                    next.remove(name);
                  } else {
                    next[name] = numeric ? num.tryParse(text) : text;
                  }
                  onChanged(next);
                },
                validator: (text) {
                  if (requiredNames.contains(name) &&
                      (text == null || text.isEmpty)) {
                    return '此参数为必填项';
                  }
                  if (text == null || text.isEmpty || !numeric) return null;
                  final number = num.tryParse(text);
                  if (number == null) return '请输入有效数字';
                  final minimum = property['minimum'] as num?;
                  final maximum = property['maximum'] as num?;
                  if (minimum != null && number < minimum) {
                    return '不能小于 $minimum';
                  }
                  if (maximum != null && number > maximum) {
                    return '不能大于 $maximum';
                  }
                  return null;
                },
              ),
            );
          })
          .toList(growable: false),
    );
  }
}

StatusBadge _deploymentStatus(String status) => StatusBadge(
  label: _statusLabel(status),
  kind: status == 'succeeded'
      ? StatusKind.success
      : status == 'queued' || status == 'running' || status == 'canceling'
      ? StatusKind.warning
      : StatusKind.neutral,
);

class _InfoLine extends StatelessWidget {
  const _InfoLine({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 8),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        SizedBox(width: 88, child: Text(label)),
        Expanded(child: SelectableText(value)),
      ],
    ),
  );
}

Map<String, Object?> _asMap(Object? value) => value is Map
    ? value.map((key, value) => MapEntry(key.toString(), value))
    : const <String, Object?>{};

Map<String, Object?> _schemaDefaults(Object? schema) {
  final properties = _asMap(_asMap(schema)['properties']);
  final result = <String, Object?>{};
  for (final entry in properties.entries) {
    final property = _asMap(entry.value);
    if (property.containsKey('default')) {
      result[entry.key] = property['default'];
    } else if (property['type'] == 'boolean') {
      result[entry.key] = false;
    }
  }
  return result;
}

String _newIdempotencyKey(String prefix) {
  final random = Random.secure();
  final bytes = List<int>.generate(16, (_) => random.nextInt(256));
  return '$prefix-${bytes.map((byte) => byte.toRadixString(16).padLeft(2, '0')).join()}';
}

String _statusLabel(String status) => switch (status) {
  'queued' => '排队中',
  'running' => '运行中',
  'canceling' => '取消中',
  'succeeded' => '成功',
  'failed' => '失败',
  'canceled' => '已取消',
  'interrupted' => '执行中断',
  _ => status,
};

String _formatTime(String value) =>
    DateTime.tryParse(value)?.toLocal().toString().substring(0, 16) ?? value;

String _connectionLabel(SseConnectionState state) => switch (state) {
  SseConnectionState.connecting => '连接中',
  SseConnectionState.open => '实时连接',
  SseConnectionState.reconnecting => '正在重连',
  SseConnectionState.ended => '已结束',
};

IconData _connectionIcon(SseConnectionState state) => switch (state) {
  SseConnectionState.open => Icons.cloud_done_outlined,
  SseConnectionState.ended => Icons.cloud_off_outlined,
  _ => Icons.sync,
};
