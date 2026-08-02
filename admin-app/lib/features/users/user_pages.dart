import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../app/providers.dart';
import '../shared/cursor_collection.dart';
import '../shared/mobile_widgets.dart';
import 'user_providers.dart';

class UsersPage extends ConsumerWidget {
  const UsersPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(usersProvider);
    return MobilePageScaffold(
      title: '用户管理',
      scrollable: false,
      actions: <Widget>[
        IconButton(
          tooltip: '新增用户',
          onPressed: () => context.go('/profile/users/new'),
          icon: const Icon(Icons.person_add_outlined),
        ),
      ],
      child: _UserCollection(state: state),
    );
  }
}

class _UserCollection extends ConsumerWidget {
  const _UserCollection({required this.state});
  final CursorCollectionState<UserResponse> state;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = ref.read(usersProvider.notifier);
    if (state.loading && state.items.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    if (state.error != null && state.items.isEmpty) {
      return MobileStateView.error(
        state.error!,
        onRetry: () => controller.refresh(),
      );
    }
    if (state.items.isEmpty) {
      return MobileStateView(
        title: '还没有普通用户',
        message: '使用右上角按钮由管理员分配账号。',
        icon: Icons.group_outlined,
        onRetry: () => context.go('/profile/users/new'),
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
              contentPadding: EdgeInsets.zero,
              leading: const Icon(Icons.error_outline),
              title: const Text('部分用户加载失败'),
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
          final user = state.items[itemIndex];
          return MobileResourceRow(
            icon: Icons.person_outline,
            title: user.displayName,
            subtitle:
                '@${user.username} · ${user.identity == 'administrator' ? '管理员' : '普通用户'}',
            status: StatusBadge(
              label: user.status == 'active' ? '启用' : '停用',
              kind: user.status == 'active'
                  ? StatusKind.success
                  : StatusKind.neutral,
            ),
            onTap: () => context.go('/profile/users/${user.id}'),
          );
        },
      ),
    );
  }
}

class NewUserPage extends ConsumerStatefulWidget {
  const NewUserPage({super.key});

  @override
  ConsumerState<NewUserPage> createState() => _NewUserPageState();
}

class _NewUserPageState extends ConsumerState<NewUserPage> {
  final formKey = GlobalKey<FormState>();
  final username = TextEditingController();
  final displayName = TextEditingController();
  final email = TextEditingController();
  final password = TextEditingController();
  bool saving = false;
  Object? error;

  @override
  void dispose() {
    username.dispose();
    displayName.dispose();
    email.dispose();
    password.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => MobilePageScaffold(
    title: '新增用户',
    child: Form(
      key: formKey,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          TextFormField(
            key: const ValueKey<String>('new-user-username'),
            controller: username,
            enabled: !saving,
            decoration: const InputDecoration(labelText: '用户名'),
            validator: _required,
          ),
          const SizedBox(height: 14),
          TextFormField(
            controller: displayName,
            enabled: !saving,
            maxLength: 120,
            decoration: const InputDecoration(labelText: '显示名称'),
          ),
          const SizedBox(height: 14),
          TextFormField(
            controller: email,
            enabled: !saving,
            keyboardType: TextInputType.emailAddress,
            decoration: const InputDecoration(labelText: '邮箱（可选）'),
          ),
          const SizedBox(height: 14),
          TextFormField(
            key: const ValueKey<String>('new-user-password'),
            controller: password,
            enabled: !saving,
            obscureText: true,
            decoration: const InputDecoration(labelText: '初始密码'),
            validator: (value) {
              if (value == null || value.length < 12) return '初始密码至少 12 位';
              return null;
            },
          ),
          const SizedBox(height: 8),
          const Text('账号由管理员直接创建，初始密码请通过系统外安全渠道交付。'),
          if (error != null) ...<Widget>[
            const SizedBox(height: 12),
            MobileStateView.error(error!, onRetry: _save),
          ],
          const SizedBox(height: 22),
          FilledButton.icon(
            onPressed: saving ? null : _save,
            icon: saving
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.person_add_outlined),
            label: Text(saving ? '正在创建' : '创建普通用户'),
          ),
        ],
      ),
    ),
  );

  String? _required(String? value) =>
      value == null || value.trim().isEmpty ? '此字段不能为空' : null;

  Future<void> _save() async {
    if (formKey.currentState?.validate() != true) return;
    setState(() {
      saving = true;
      error = null;
    });
    try {
      final created = await ref
          .read(mobileDataGatewayProvider)
          .createUser(
            username: username.text.trim(),
            displayName: displayName.text.trim(),
            email: email.text.trim(),
            password: password.text,
          );
      ref.invalidate(usersProvider);
      if (mounted) context.go('/profile/users/${created.id}');
    } catch (caught) {
      error = caught;
      if (mounted) setState(() {});
    } finally {
      if (mounted) setState(() => saving = false);
    }
  }
}

class UserDetailPage extends ConsumerStatefulWidget {
  const UserDetailPage({required this.id, super.key});
  final String id;

  @override
  ConsumerState<UserDetailPage> createState() => _UserDetailPageState();
}

class _UserDetailPageState extends ConsumerState<UserDetailPage> {
  bool saving = false;
  Object? actionError;

  @override
  Widget build(BuildContext context) {
    final value = ref.watch(userProvider(widget.id));
    return MobilePageScaffold(
      title: '用户详情',
      child: value.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, _) => MobileStateView.error(
          error,
          onRetry: () => ref.invalidate(userProvider(widget.id)),
        ),
        data: (user) => Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Row(
              children: <Widget>[
                const CircleAvatar(
                  radius: 25,
                  child: Icon(Icons.person_outline),
                ),
                const SizedBox(width: 14),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        user.displayName,
                        style: Theme.of(context).textTheme.titleLarge,
                      ),
                      Text('@${user.username}'),
                    ],
                  ),
                ),
                StatusBadge(
                  label: user.status == 'active' ? '启用' : '停用',
                  kind: user.status == 'active'
                      ? StatusKind.success
                      : StatusKind.neutral,
                ),
              ],
            ),
            const SectionHeader('账号信息'),
            _UserLine(
              label: '身份',
              value: user.identity == 'administrator' ? '管理员' : '普通用户',
            ),
            _UserLine(label: '邮箱', value: user.email ?? '未设置'),
            if (actionError != null) ...<Widget>[
              const SizedBox(height: 12),
              MobileStateView.error(
                actionError!,
                onRetry: () => _changeStatus(user),
              ),
            ],
            if (user.identity != 'administrator') ...<Widget>[
              const SizedBox(height: 24),
              OutlinedButton.icon(
                onPressed: saving ? null : () => _confirmStatus(user),
                icon: Icon(
                  user.status == 'active'
                      ? Icons.person_off_outlined
                      : Icons.person_outline,
                ),
                label: Text(user.status == 'active' ? '停用用户' : '启用用户'),
                style: user.status == 'active'
                    ? OutlinedButton.styleFrom(
                        foregroundColor: Theme.of(context).colorScheme.error,
                        side: BorderSide(
                          color: Theme.of(context).colorScheme.error,
                        ),
                        minimumSize: const Size(44, 48),
                      )
                    : null,
              ),
            ],
          ],
        ),
      ),
    );
  }

  Future<void> _confirmStatus(UserResponse user) async {
    final active = user.status == 'active';
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(active ? '停用这个用户？' : '启用这个用户？'),
        content: Text(active ? '停用后该用户的现有会话将被撤销。' : '启用后用户可以重新登录。'),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            style: active
                ? FilledButton.styleFrom(
                    backgroundColor: Theme.of(context).colorScheme.error,
                  )
                : null,
            child: Text(active ? '确认停用' : '确认启用'),
          ),
        ],
      ),
    );
    if (confirmed == true) await _changeStatus(user);
  }

  Future<void> _changeStatus(UserResponse user) async {
    setState(() {
      saving = true;
      actionError = null;
    });
    try {
      await ref
          .read(mobileDataGatewayProvider)
          .updateUserStatus(
            user,
            user.status == 'active' ? 'disabled' : 'active',
          );
      ref.invalidate(userProvider(widget.id));
      ref.invalidate(usersProvider);
    } catch (error) {
      actionError = error;
    } finally {
      if (mounted) setState(() => saving = false);
    }
  }
}

class ForbiddenPage extends StatelessWidget {
  const ForbiddenPage({super.key});

  @override
  Widget build(BuildContext context) => MobilePageScaffold(
    title: '权限不足',
    child: MobileStateView(
      title: '此功能仅管理员可用',
      message: '当前账号没有系统管理权限。',
      icon: Icons.lock_outline,
      onRetry: () => context.go('/profile'),
    ),
  );
}

class NotFoundPage extends StatelessWidget {
  const NotFoundPage({super.key});

  @override
  Widget build(BuildContext context) => MobilePageScaffold(
    title: '页面不存在',
    child: MobileStateView(
      title: '找不到这个页面',
      icon: Icons.search_off_outlined,
      onRetry: () => context.go('/overview'),
    ),
  );
}

class _UserLine extends StatelessWidget {
  const _UserLine({required this.label, required this.value});
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 9),
    child: Row(
      children: <Widget>[
        SizedBox(width: 88, child: Text(label)),
        Expanded(child: Text(value)),
      ],
    ),
  );
}
