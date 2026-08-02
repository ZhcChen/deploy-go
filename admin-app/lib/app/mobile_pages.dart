import 'package:flutter/material.dart';

enum ResourceSegment { applications, nodes }

class OverviewRootPage extends StatelessWidget {
  const OverviewRootPage({super.key});

  @override
  Widget build(BuildContext context) => const _RootPage(
    title: '概览',
    child: _Placeholder(
      icon: Icons.monitor_heart_outlined,
      title: '部署运行概览',
      message: '应用、节点和最近部署将在下一阶段接入。',
    ),
  );
}

class ResourcesRootPage extends StatefulWidget {
  const ResourcesRootPage({super.key});

  @override
  State<ResourcesRootPage> createState() => _ResourcesRootPageState();
}

class _ResourcesRootPageState extends State<ResourcesRootPage> {
  ResourceSegment segment = ResourceSegment.applications;

  @override
  Widget build(BuildContext context) => _RootPage(
    title: '资源',
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        SegmentedButton<ResourceSegment>(
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
          onSelectionChanged: (value) => setState(() => segment = value.first),
        ),
        const SizedBox(height: 28),
        _Placeholder(
          icon: segment == ResourceSegment.applications
              ? Icons.inventory_2_outlined
              : Icons.dns_outlined,
          title: segment == ResourceSegment.applications ? '应用资源' : '节点资源',
          message: '资源列表将在下一阶段接入正式 API。',
        ),
      ],
    ),
  );
}

class DeploymentsRootPage extends StatelessWidget {
  const DeploymentsRootPage({super.key});

  @override
  Widget build(BuildContext context) => const _RootPage(
    key: ValueKey<String>('deployment-root'),
    title: '部署',
    child: _Placeholder(
      icon: Icons.rocket_launch_outlined,
      title: '部署任务',
      message: '部署预览、日志和恢复将在部署单元接入。',
    ),
  );
}

class ProfileRootPage extends StatelessWidget {
  const ProfileRootPage({required this.displayName, super.key});

  final String displayName;

  @override
  Widget build(BuildContext context) => Scaffold(
    body: SafeArea(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Container(
            padding: const EdgeInsets.fromLTRB(20, 24, 20, 20),
            color: Colors.white,
            child: Row(
              children: <Widget>[
                const CircleAvatar(
                  radius: 26,
                  child: Icon(Icons.person_outline),
                ),
                const SizedBox(width: 14),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        displayName,
                        style: Theme.of(context).textTheme.titleLarge,
                      ),
                      const SizedBox(height: 4),
                      const Text('Deploy Go 账号'),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const Expanded(
            child: _Placeholder(
              icon: Icons.manage_accounts_outlined,
              title: '账号与偏好',
              message: '个人资料与管理员入口将在下一阶段接入。',
            ),
          ),
        ],
      ),
    ),
  );
}

class _RootPage extends StatelessWidget {
  const _RootPage({required this.title, required this.child, super.key});
  final String title;
  final Widget child;

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: Text(title)),
    body: SafeArea(
      top: false,
      child: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
        child: child,
      ),
    ),
  );
}

class _Placeholder extends StatelessWidget {
  const _Placeholder({
    required this.icon,
    required this.title,
    required this.message,
  });
  final IconData icon;
  final String title;
  final String message;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 36, horizontal: 12),
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Icon(icon, size: 36),
        const SizedBox(height: 14),
        Text(
          title,
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.titleLarge,
        ),
        const SizedBox(height: 8),
        Text(message, textAlign: TextAlign.center),
      ],
    ),
  );
}
