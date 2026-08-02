# Deploy Go 管理 App

Flutter 正式移动管理端。工程使用 Riverpod、`go_router`、Dio/CookieJar，并通过 Android Keystore 与 iOS Keychain 保存 Cookie 和 CSRF token。

当前已提供概览、应用与节点只读信息、个人资料、通知偏好和管理员用户管理。SSH 凭证生成、节点绑定与 Host key 确认仍由 Web 管理端负责。

从仓库根目录运行：

```bash
make admin-app-get
make admin-app-check
make admin-app
```

本地联调必须通过 `--dart-define` 显式设置 API 地址和与服务端完全一致的 Origin，具体命令见 `docs/runbooks/local-development.md`。这不授权连接真实节点或执行真实部署。
