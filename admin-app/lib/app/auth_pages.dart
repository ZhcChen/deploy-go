import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'providers.dart';

class LoginPage extends ConsumerStatefulWidget {
  const LoginPage({super.key});

  @override
  ConsumerState<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends ConsumerState<LoginPage> {
  final username = TextEditingController();
  final password = TextEditingController();

  @override
  void dispose() {
    username.dispose();
    password.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => _AuthScaffold(
    title: '登录 Deploy Go',
    message: ref.watch(sessionControllerProvider).message,
    children: <Widget>[
      TextField(
        controller: username,
        decoration: const InputDecoration(labelText: '用户名或邮箱'),
        textInputAction: TextInputAction.next,
      ),
      TextField(
        controller: password,
        decoration: const InputDecoration(labelText: '密码'),
        obscureText: true,
        onSubmitted: (_) => _submit(),
      ),
      FilledButton(onPressed: _submit, child: const Text('登录')),
    ],
  );

  void _submit() {
    ref
        .read(sessionControllerProvider.notifier)
        .login(username.text, password.text);
  }
}

class SetupPage extends ConsumerStatefulWidget {
  const SetupPage({super.key});

  @override
  ConsumerState<SetupPage> createState() => _SetupPageState();
}

class _SetupPageState extends ConsumerState<SetupPage> {
  final username = TextEditingController();
  final displayName = TextEditingController();
  final password = TextEditingController();

  @override
  void dispose() {
    username.dispose();
    displayName.dispose();
    password.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => _AuthScaffold(
    title: '初始化管理员',
    message: ref.watch(sessionControllerProvider).message,
    children: <Widget>[
      TextField(
        key: const ValueKey<String>('setup-username'),
        controller: username,
        decoration: const InputDecoration(labelText: '用户名'),
      ),
      TextField(
        key: const ValueKey<String>('setup-display-name'),
        controller: displayName,
        decoration: const InputDecoration(labelText: '显示名称'),
      ),
      TextField(
        key: const ValueKey<String>('setup-password'),
        controller: password,
        decoration: const InputDecoration(labelText: '初始密码'),
        obscureText: true,
      ),
      FilledButton(
        onPressed: () {
          ref
              .read(sessionControllerProvider.notifier)
              .setup(
                username: username.text,
                password: password.text,
                displayName: displayName.text,
              );
        },
        child: const Text('完成初始化'),
      ),
    ],
  );
}

class _AuthScaffold extends StatelessWidget {
  const _AuthScaffold({
    required this.title,
    required this.children,
    this.message,
  });
  final String title;
  final List<Widget> children;
  final String? message;

  @override
  Widget build(BuildContext context) => Scaffold(
    body: SafeArea(
      child: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 420),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                const Icon(Icons.terminal, size: 42),
                const SizedBox(height: 18),
                Text(
                  title,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.headlineSmall,
                ),
                if (message != null) ...<Widget>[
                  const SizedBox(height: 16),
                  Text(
                    message!,
                    textAlign: TextAlign.center,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ],
                const SizedBox(height: 28),
                for (final child in children) ...<Widget>[
                  child,
                  const SizedBox(height: 14),
                ],
              ],
            ),
          ),
        ),
      ),
    ),
  );
}
