import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../api/mobile_data_gateway.dart';

class MobilePageScaffold extends StatelessWidget {
  const MobilePageScaffold({
    required this.title,
    required this.child,
    this.actions,
    this.scrollable = true,
    super.key,
  });

  final String title;
  final Widget child;
  final List<Widget>? actions;
  final bool scrollable;

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: Text(title), actions: actions),
    body: SafeArea(
      top: false,
      child: scrollable
          ? SingleChildScrollView(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
              child: child,
            )
          : child,
    ),
  );
}

class MobileStateView extends StatelessWidget {
  const MobileStateView({
    required this.title,
    this.message,
    this.icon = Icons.inbox_outlined,
    this.onRetry,
    super.key,
  });

  final String title;
  final String? message;
  final IconData icon;
  final VoidCallback? onRetry;

  factory MobileStateView.error(Object error, {required VoidCallback onRetry}) {
    final failure = error is ApiFailureException ? error.failure : null;
    final requestId = failure?.requestId ?? '';
    return MobileStateView(
      title: failure?.message ?? '暂时无法加载',
      message: requestId.isEmpty ? '请稍后重试' : 'Request ID: $requestId',
      icon: Icons.error_outline,
      onRetry: onRetry,
    );
  }

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 48),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(icon, size: 38),
          const SizedBox(height: 14),
          Text(
            title,
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.titleLarge,
          ),
          if (message != null) ...<Widget>[
            const SizedBox(height: 8),
            Text(message!, textAlign: TextAlign.center),
          ],
          if (message?.startsWith('Request ID: ') ?? false) ...<Widget>[
            const SizedBox(height: 10),
            TextButton.icon(
              onPressed: () => Clipboard.setData(
                ClipboardData(text: message!.substring('Request ID: '.length)),
              ),
              icon: const Icon(Icons.copy_outlined),
              label: const Text('复制 Request ID'),
            ),
          ],
          if (onRetry != null) ...<Widget>[
            const SizedBox(height: 18),
            OutlinedButton.icon(
              onPressed: onRetry,
              icon: const Icon(Icons.refresh),
              label: const Text('重试'),
            ),
          ],
        ],
      ),
    ),
  );
}

class MobileResourceRow extends StatelessWidget {
  const MobileResourceRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.status,
    this.onTap,
    super.key,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final StatusBadge status;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) => ListTile(
    minTileHeight: 68,
    contentPadding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
    leading: CircleAvatar(child: Icon(icon, size: 20)),
    title: Text(title, maxLines: 1, overflow: TextOverflow.ellipsis),
    subtitle: Text(subtitle, maxLines: 2, overflow: TextOverflow.ellipsis),
    trailing: status,
    onTap: onTap,
  );
}

class StatusBadge extends StatelessWidget {
  const StatusBadge({required this.label, required this.kind, super.key});

  final String label;
  final StatusKind kind;

  @override
  Widget build(BuildContext context) {
    final colors = switch (kind) {
      StatusKind.success => (const Color(0xFF1A7F37), const Color(0xFFDAFBE1)),
      StatusKind.warning => (const Color(0xFF9A6700), const Color(0xFFFFF8C5)),
      StatusKind.danger => (const Color(0xFFCF222E), const Color(0xFFFFEBE9)),
      StatusKind.neutral => (const Color(0xFF57606A), const Color(0xFFF0F2F4)),
    };
    return Container(
      constraints: const BoxConstraints(minHeight: 28),
      padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 5),
      decoration: BoxDecoration(
        color: colors.$2,
        borderRadius: BorderRadius.circular(14),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: colors.$1,
          fontSize: 12,
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }
}

enum StatusKind { success, warning, danger, neutral }

class SectionHeader extends StatelessWidget {
  const SectionHeader(this.title, {super.key});
  final String title;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(4, 20, 4, 8),
    child: Text(
      title,
      style: Theme.of(
        context,
      ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
    ),
  );
}

class SummaryTile extends StatelessWidget {
  const SummaryTile({
    required this.label,
    required this.value,
    required this.icon,
    super.key,
  });

  final String label;
  final String value;
  final IconData icon;

  @override
  Widget build(BuildContext context) => Semantics(
    label: '$label $value',
    excludeSemantics: true,
    child: Container(
      constraints: const BoxConstraints(minHeight: 92),
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        border: Border.all(color: Theme.of(context).colorScheme.outline),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Icon(icon, size: 20),
          const Spacer(),
          Text(value, style: Theme.of(context).textTheme.titleLarge),
          Text(label),
        ],
      ),
    ),
  );
}
