import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../routing/app_router.dart';
import '../theme/app_theme.dart';
import 'providers.dart';

class DeployGoApp extends ConsumerStatefulWidget {
  const DeployGoApp({super.key});

  @override
  ConsumerState<DeployGoApp> createState() => _DeployGoAppState();
}

class _DeployGoAppState extends ConsumerState<DeployGoApp> {
  late final GoRouter router;

  @override
  void initState() {
    super.initState();
    router = createAppRouter(ref.read(sessionControllerProvider.notifier));
  }

  @override
  void dispose() {
    router.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => MaterialApp.router(
    title: 'Deploy Go',
    debugShowCheckedModeBanner: false,
    theme: AppTheme.light,
    routerConfig: router,
  );
}
