import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../app/providers.dart';
import '../shared/mobile_widgets.dart';
import 'profile_providers.dart';

class ProfileRootPage extends ConsumerWidget {
  const ProfileRootPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionControllerProvider).session;
    final user = session?.user;
    final administrator = user?.identity == 'administrator';
    return Scaffold(
      body: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Container(
              key: const ValueKey<String>('profile-identity-header'),
              padding: const EdgeInsets.fromLTRB(20, 24, 20, 20),
              color: Theme.of(context).colorScheme.surface,
              child: Row(
                children: <Widget>[
                  const CircleAvatar(
                    radius: 27,
                    child: Icon(Icons.person_outline),
                  ),
                  const SizedBox(width: 14),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          user?.displayName ?? '账号',
                          style: Theme.of(context).textTheme.titleLarge,
                        ),
                        const SizedBox(height: 4),
                        Text('@${user?.username ?? '-'}'),
                      ],
                    ),
                  ),
                  StatusBadge(
                    label: administrator ? '管理员' : '普通用户',
                    kind: StatusKind.neutral,
                  ),
                ],
              ),
            ),
            Expanded(
              child: ListView(
                padding: const EdgeInsets.fromLTRB(16, 4, 16, 28),
                children: <Widget>[
                  const SectionHeader('账号'),
                  _ProfileEntry(
                    icon: Icons.manage_accounts_outlined,
                    title: '个人资料',
                    onTap: () => context.go('/profile/details'),
                  ),
                  _ProfileEntry(
                    icon: Icons.tune_outlined,
                    title: '通知与偏好',
                    onTap: () => context.go('/profile/preferences'),
                  ),
                  if (administrator) ...<Widget>[
                    const SectionHeader('系统管理'),
                    _ProfileEntry(
                      key: const ValueKey<String>('user-management-entry'),
                      icon: Icons.group_outlined,
                      title: '用户管理',
                      onTap: () => context.go('/profile/users'),
                    ),
                  ],
                  const SectionHeader('其他'),
                  _ProfileEntry(
                    icon: Icons.info_outline,
                    title: '关于 Deploy Go',
                    onTap: () => context.go('/profile/about'),
                  ),
                  const SizedBox(height: 28),
                  OutlinedButton.icon(
                    key: const ValueKey<String>('logout-button'),
                    onPressed: () => _confirmLogout(context, ref),
                    icon: const Icon(Icons.logout),
                    label: const Text('退出登录'),
                    style: OutlinedButton.styleFrom(
                      foregroundColor: Theme.of(context).colorScheme.error,
                      side: BorderSide(
                        color: Theme.of(context).colorScheme.error,
                      ),
                      minimumSize: const Size(44, 48),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _confirmLogout(BuildContext context, WidgetRef ref) async {
    final confirmed = await showModalBottomSheet<bool>(
      context: context,
      showDragHandle: true,
      builder: (context) => SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 4, 20, 20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              Text('退出当前账号？', style: Theme.of(context).textTheme.titleLarge),
              const SizedBox(height: 8),
              const Text('退出后将清除本机 Cookie 和 CSRF 会话信息。'),
              const SizedBox(height: 20),
              FilledButton(
                onPressed: () => Navigator.pop(context, true),
                style: FilledButton.styleFrom(
                  backgroundColor: Theme.of(context).colorScheme.error,
                ),
                child: const Text('确认退出'),
              ),
              TextButton(
                onPressed: () => Navigator.pop(context, false),
                child: const Text('取消'),
              ),
            ],
          ),
        ),
      ),
    );
    if (confirmed == true) {
      await ref.read(sessionControllerProvider.notifier).logout();
    }
  }
}

class _ProfileEntry extends StatelessWidget {
  const _ProfileEntry({
    required this.icon,
    required this.title,
    required this.onTap,
    super.key,
  });
  final IconData icon;
  final String title;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => ListTile(
    minTileHeight: 56,
    leading: Icon(icon),
    title: Text(title),
    trailing: const Icon(Icons.chevron_right),
    onTap: onTap,
  );
}

class ProfileDetailsPage extends ConsumerStatefulWidget {
  const ProfileDetailsPage({super.key});

  @override
  ConsumerState<ProfileDetailsPage> createState() => _ProfileDetailsPageState();
}

class _ProfileDetailsPageState extends ConsumerState<ProfileDetailsPage> {
  final displayName = TextEditingController();
  bool initialized = false;
  bool saving = false;
  Object? error;

  @override
  void dispose() {
    displayName.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final profile = ref.watch(profileProvider);
    return MobilePageScaffold(
      title: '个人资料',
      child: profile.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, _) => MobileStateView.error(
          error,
          onRetry: () => ref.invalidate(profileProvider),
        ),
        data: (user) {
          if (!initialized) {
            displayName.text = user.displayName;
            initialized = true;
          }
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              TextFormField(
                controller: displayName,
                enabled: !saving,
                maxLength: 120,
                decoration: const InputDecoration(labelText: '显示名称'),
                onChanged: (_) => setState(() {}),
              ),
              const SizedBox(height: 10),
              _ReadOnlyLine(label: '用户名', value: '@${user.username}'),
              _ReadOnlyLine(label: '邮箱', value: user.email ?? '未设置'),
              if (error != null) ...<Widget>[
                const SizedBox(height: 12),
                MobileStateView.error(error!, onRetry: _save),
              ],
              const SizedBox(height: 20),
              FilledButton.icon(
                key: const ValueKey<String>('save-profile-button'),
                onPressed: saving || displayName.text.trim() == user.displayName
                    ? null
                    : _save,
                icon: saving
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.save_outlined),
                label: Text(saving ? '正在保存' : '保存资料'),
              ),
            ],
          );
        },
      ),
    );
  }

  Future<void> _save() async {
    final value = displayName.text.trim();
    if (value.isEmpty) return;
    setState(() {
      saving = true;
      error = null;
    });
    try {
      final saved = await ref
          .read(mobileDataGatewayProvider)
          .updateProfile(value);
      ref.read(sessionControllerProvider.notifier).applyUser(saved);
      ref.invalidate(profileProvider);
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('个人资料已更新')));
      }
    } catch (caught) {
      error = caught;
    } finally {
      if (mounted) setState(() => saving = false);
    }
  }
}

class PreferencesPage extends ConsumerStatefulWidget {
  const PreferencesPage({super.key});

  @override
  ConsumerState<PreferencesPage> createState() => _PreferencesPageState();
}

class _PreferencesPageState extends ConsumerState<PreferencesPage> {
  UserPreferencesResponse? draft;
  bool saving = false;
  Object? error;

  @override
  Widget build(BuildContext context) {
    final source = ref.watch(preferencesProvider);
    return MobilePageScaffold(
      title: '通知与偏好',
      child: source.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, _) => MobileStateView.error(
          error,
          onRetry: () => ref.invalidate(preferencesProvider),
        ),
        data: (value) {
          final current = draft ?? value;
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              _PreferenceSwitch(
                title: '部署失败',
                value: current.notifyDeploymentFailed,
                onChanged: saving
                    ? null
                    : (enabled) => _update(
                        current.rebuild(
                          (builder) => builder.notifyDeploymentFailed = enabled,
                        ),
                      ),
              ),
              _PreferenceSwitch(
                title: '部署完成',
                value: current.notifyDeploymentCompleted,
                onChanged: saving
                    ? null
                    : (enabled) => _update(
                        current.rebuild(
                          (builder) =>
                              builder.notifyDeploymentCompleted = enabled,
                        ),
                      ),
              ),
              _PreferenceSwitch(
                title: '节点异常',
                value: current.notifyNodeUnhealthy,
                onChanged: saving
                    ? null
                    : (enabled) => _update(
                        current.rebuild(
                          (builder) => builder.notifyNodeUnhealthy = enabled,
                        ),
                      ),
              ),
              _PreferenceSwitch(
                title: '默认跟随部署日志',
                value: current.followLogs,
                onChanged: saving
                    ? null
                    : (enabled) => _update(
                        current.rebuild(
                          (builder) => builder.followLogs = enabled,
                        ),
                      ),
              ),
              const SectionHeader('时间格式'),
              SegmentedButton<String>(
                segments: const <ButtonSegment<String>>[
                  ButtonSegment(value: '24h', label: Text('24 小时')),
                  ButtonSegment(value: '12h', label: Text('12 小时')),
                ],
                selected: <String>{current.timeFormat},
                onSelectionChanged: saving
                    ? null
                    : (selection) => _update(
                        current.rebuild(
                          (builder) => builder.timeFormat = selection.first,
                        ),
                      ),
              ),
              if (error != null) ...<Widget>[
                const SizedBox(height: 12),
                MobileStateView.error(error!, onRetry: _save),
              ],
              const SizedBox(height: 24),
              FilledButton.icon(
                key: const ValueKey<String>('save-preferences-button'),
                onPressed: saving || draft == null ? null : _save,
                icon: saving
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.save_outlined),
                label: Text(saving ? '正在保存' : '保存偏好'),
              ),
            ],
          );
        },
      ),
    );
  }

  void _update(UserPreferencesResponse value) => setState(() => draft = value);

  Future<void> _save() async {
    final value = draft;
    if (value == null) return;
    setState(() {
      saving = true;
      error = null;
    });
    try {
      await ref.read(mobileDataGatewayProvider).updatePreferences(value);
      draft = null;
      ref.invalidate(preferencesProvider);
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('偏好已保存')));
      }
    } catch (caught) {
      error = caught;
    } finally {
      if (mounted) setState(() => saving = false);
    }
  }
}

class AboutPage extends StatelessWidget {
  const AboutPage({super.key});

  @override
  Widget build(BuildContext context) => const MobilePageScaffold(
    title: '关于 Deploy Go',
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Icon(Icons.terminal, size: 44),
        SizedBox(height: 16),
        Text(
          'Deploy Go',
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 22, fontWeight: FontWeight.w700),
        ),
        SizedBox(height: 22),
        Text('轻量部署平台只负责执行应用自有脚本并回馈结果，不接管脚本内部的构建、切流或回滚过程。'),
      ],
    ),
  );
}

class _ReadOnlyLine extends StatelessWidget {
  const _ReadOnlyLine({required this.label, required this.value});
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 10),
    child: Row(
      children: <Widget>[
        SizedBox(width: 88, child: Text(label)),
        Expanded(child: Text(value)),
      ],
    ),
  );
}

class _PreferenceSwitch extends StatelessWidget {
  const _PreferenceSwitch({
    required this.title,
    required this.value,
    required this.onChanged,
  });
  final String title;
  final bool value;
  final ValueChanged<bool>? onChanged;

  @override
  Widget build(BuildContext context) => SwitchListTile(
    contentPadding: EdgeInsets.zero,
    minTileHeight: 56,
    title: Text(title),
    value: value,
    onChanged: onChanged,
  );
}
