import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../app/auth_pages.dart';
import '../app/mobile_pages.dart';
import '../app/providers.dart';
import '../features/overview/overview_page.dart';
import '../features/profile/profile_pages.dart';
import '../features/resources/resources_pages.dart';
import '../features/users/user_pages.dart';

GoRouter createAppRouter(SessionController session, {String? initialLocation}) {
  final refresh = _SessionRefresh(session);
  return GoRouter(
    initialLocation: initialLocation,
    refreshListenable: refresh,
    redirect: (context, state) {
      final phase = session.current.phase;
      final authPath =
          state.matchedLocation == '/login' ||
          state.matchedLocation == '/setup' ||
          state.matchedLocation == '/boot';
      final administratorPath = state.matchedLocation.startsWith(
        '/profile/users',
      );
      final pendingLocation = _pendingLocation(state);
      return switch (phase) {
        SessionPhase.bootstrapping =>
          state.matchedLocation == '/boot'
              ? null
              : _authLocation('/boot', pendingLocation),
        SessionPhase.setupRequired =>
          state.matchedLocation == '/setup'
              ? null
              : _authLocation('/setup', pendingLocation),
        SessionPhase.unauthenticated || SessionPhase.failure =>
          state.matchedLocation == '/login'
              ? null
              : _authLocation('/login', pendingLocation),
        SessionPhase.authenticated =>
          authPath
              ? pendingLocation ?? '/overview'
              : administratorPath &&
                    session.current.session?.user.identity != 'administrator'
              ? '/forbidden'
              : null,
      };
    },
    errorBuilder: (context, state) => const NotFoundPage(),
    routes: <RouteBase>[
      GoRoute(path: '/boot', builder: (context, state) => const _BootPage()),
      GoRoute(path: '/setup', builder: (context, state) => const SetupPage()),
      GoRoute(path: '/login', builder: (context, state) => const LoginPage()),
      GoRoute(
        path: '/forbidden',
        builder: (context, state) => const ForbiddenPage(),
      ),
      StatefulShellRoute.indexedStack(
        builder: (context, state, shell) => _MobileShell(shell: shell),
        branches: <StatefulShellBranch>[
          StatefulShellBranch(
            routes: <RouteBase>[
              GoRoute(
                path: '/overview',
                builder: (context, state) => const OverviewPage(),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: <RouteBase>[
              GoRoute(
                path: '/resources',
                builder: (context, state) => const ResourcesPage(),
                routes: <RouteBase>[
                  GoRoute(
                    path: 'applications/:id',
                    builder: (context, state) =>
                        ApplicationDetailPage(id: state.pathParameters['id']!),
                  ),
                  GoRoute(
                    path: 'nodes/:id',
                    builder: (context, state) =>
                        NodeDetailPage(id: state.pathParameters['id']!),
                  ),
                ],
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
                builder: (context, state) => const ProfileRootPage(),
                routes: <RouteBase>[
                  GoRoute(
                    path: 'details',
                    builder: (context, state) => const ProfileDetailsPage(),
                  ),
                  GoRoute(
                    path: 'preferences',
                    builder: (context, state) => const PreferencesPage(),
                  ),
                  GoRoute(
                    path: 'about',
                    builder: (context, state) => const AboutPage(),
                  ),
                  GoRoute(
                    path: 'users',
                    builder: (context, state) => const UsersPage(),
                    routes: <RouteBase>[
                      GoRoute(
                        path: 'new',
                        builder: (context, state) => const NewUserPage(),
                      ),
                      GoRoute(
                        path: ':id',
                        builder: (context, state) =>
                            UserDetailPage(id: state.pathParameters['id']!),
                      ),
                    ],
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    ],
  );
}

String _authLocation(String path, String? returnTo) => returnTo == null
    ? path
    : Uri(
        path: path,
        queryParameters: <String, String>{'returnTo': returnTo},
      ).toString();

String? _pendingLocation(GoRouterState state) {
  final carried = state.uri.queryParameters['returnTo'];
  final candidate = carried ?? state.uri.toString();
  final uri = Uri.tryParse(candidate);
  if (uri == null ||
      !candidate.startsWith('/') ||
      candidate.startsWith('//') ||
      uri.hasAuthority ||
      uri.scheme.isNotEmpty) {
    return null;
  }
  if (uri.path == '/' ||
      uri.path == '/boot' ||
      uri.path == '/setup' ||
      uri.path == '/login') {
    return null;
  }
  return candidate;
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
