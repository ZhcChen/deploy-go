import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../app/auth_pages.dart';
import '../app/mobile_pages.dart';
import '../app/providers.dart';

GoRouter createAppRouter(SessionController session) {
  final refresh = _SessionRefresh(session);
  return GoRouter(
    initialLocation: '/boot',
    refreshListenable: refresh,
    redirect: (context, state) {
      final phase = session.current.phase;
      final authPath =
          state.matchedLocation == '/login' ||
          state.matchedLocation == '/setup' ||
          state.matchedLocation == '/boot';
      return switch (phase) {
        SessionPhase.bootstrapping =>
          state.matchedLocation == '/boot' ? null : '/boot',
        SessionPhase.setupRequired =>
          state.matchedLocation == '/setup' ? null : '/setup',
        SessionPhase.unauthenticated || SessionPhase.failure =>
          state.matchedLocation == '/login' ? null : '/login',
        SessionPhase.authenticated => authPath ? '/overview' : null,
      };
    },
    routes: <RouteBase>[
      GoRoute(path: '/boot', builder: (context, state) => const _BootPage()),
      GoRoute(path: '/setup', builder: (context, state) => const SetupPage()),
      GoRoute(path: '/login', builder: (context, state) => const LoginPage()),
      StatefulShellRoute.indexedStack(
        builder: (context, state, shell) => _MobileShell(shell: shell),
        branches: <StatefulShellBranch>[
          StatefulShellBranch(
            routes: <RouteBase>[
              GoRoute(
                path: '/overview',
                builder: (context, state) => const OverviewRootPage(),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: <RouteBase>[
              GoRoute(
                path: '/resources',
                builder: (context, state) => const ResourcesRootPage(),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: <RouteBase>[
              GoRoute(
                path: '/deployments',
                builder: (context, state) => const DeploymentsRootPage(),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: <RouteBase>[
              GoRoute(
                path: '/profile',
                builder: (context, state) => ProfileRootPage(
                  displayName:
                      session.current.session?.user.displayName ?? '账号',
                ),
              ),
            ],
          ),
        ],
      ),
    ],
  );
}

class _SessionRefresh extends ChangeNotifier {
  _SessionRefresh(SessionController session) {
    _remove = session.addListener(
      (_) => notifyListeners(),
      fireImmediately: false,
    );
  }
  late final void Function() _remove;

  @override
  void dispose() {
    _remove();
    super.dispose();
  }
}

class _MobileShell extends StatelessWidget {
  const _MobileShell({required this.shell});
  final StatefulNavigationShell shell;

  @override
  Widget build(BuildContext context) => Scaffold(
    body: shell,
    bottomNavigationBar: NavigationBar(
      selectedIndex: shell.currentIndex,
      onDestinationSelected: (index) =>
          shell.goBranch(index, initialLocation: index == shell.currentIndex),
      destinations: const <NavigationDestination>[
        NavigationDestination(
          icon: Icon(Icons.dashboard_outlined),
          selectedIcon: Icon(Icons.dashboard),
          label: '概览',
        ),
        NavigationDestination(
          icon: Icon(Icons.widgets_outlined),
          selectedIcon: Icon(Icons.widgets),
          label: '资源',
        ),
        NavigationDestination(
          icon: Icon(Icons.rocket_launch_outlined),
          selectedIcon: Icon(Icons.rocket_launch),
          label: '部署',
        ),
        NavigationDestination(
          icon: Icon(Icons.person_outline),
          selectedIcon: Icon(Icons.person),
          label: '我的',
        ),
      ],
    ),
  );
}

class _BootPage extends StatelessWidget {
  const _BootPage();

  @override
  Widget build(BuildContext context) =>
      const Scaffold(body: Center(child: CircularProgressIndicator()));
}
