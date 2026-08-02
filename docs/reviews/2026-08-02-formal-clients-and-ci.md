# 正式客户端与 CI 最终复核

## 结论

U1-U15 已形成可独立解释的提交闭环。Web、Flutter、OpenAPI 生成、本地命令和 CI/release 配置满足计划范围；未引入角色管理、邀请、公开注册、多管理员或平台接管部署流程。

当前无未处理 P0/P1 实现问题。GitHub 托管 CI 与 release workflow dry-run 是 U14 的最终运行态门禁，推送后执行；真实节点、真实部署和生产发布不在本次验证范围。

## 验收证据

| 范围 | 结果 | 证据 |
| --- | --- | --- |
| API 与 migration | 通过 | `make api-check`；Rust 单元、集成、migration、OpenAPI 检查全部通过 |
| 双端生成 | 通过 | `make api-client-check`，临时生成无漂移 |
| Web | 通过 | lint、typecheck、46 项 Vitest、build；15 项 Playwright 包含键盘、axe、权限和部署闭环 |
| Flutter | 通过 | format、analyze、46 项 unit/widget/provider 测试；Android 15 三项 integration smoke |
| UI 设计源 | 通过 | `make ui-check`；30 项 Playwright，8050 预览可访问 |
| 跨端安全 | 通过 | 401/403/409/422/500 fixture、身份缓存清理、未授权部署清理、5 类敏感 canary 和源码/构建产物扫描 |
| 本地命令 | 通过 | Web 端口覆盖及占用报错、Web/Flutter test/build、设备 ID 缺失报错、聚合 `make check` |
| Workflow 语法 | 通过 | Ruby YAML parser；`actionlint v1.7.7` |
| Web release dry-run | 通过 | 静态构建、归档入口、解包与敏感扫描 |
| Android debug APK | 通过 | 本地构建、Android 15 安装及 smoke |
| Android unsigned AAB | 托管验证 | 本机生成的 AAB 可解包、无签名块且敏感扫描通过，但 Flutter 因 NDK 28.2 缺少 `llvm-strip` 返回失败；release workflow 在干净 Ubuntu 环境重新构建并强制检查签名块 |
| iOS secure session | 托管验证 | 本机 Xcode 26.6 build service 卡在 package loading；CI 在 `macos-15` 的可用 Simulator 执行 |

## Acceptance Examples

- AE1-AE3：setup/login 会话、SSH onboarding、preview snapshot 冲突由 API、Web 和 UI 测试覆盖。
- AE4-AE5：SSE 游标续传、去重、后台释放和恢复由双端测试及 Android lifecycle smoke 覆盖。
- AE6-AE8：权限深链、个人偏好、敏感值与身份缓存清理由跨端契约、负向测试和扫描覆盖。
- AE9：CI 执行 `make api-client-check`，任一 generated 目录漂移会失败。
- AE10：release workflow 保留 API 双架构产物，新增 Web archive、Android debug APK、unsigned release AAB 和统一 `SHA256SUMS`；Android/iOS 签名边界已写入 runbook。

## 发布边界

- Android release build 不配置 signingConfig；workflow 不读取 keystore 或签名密码。
- iOS 只执行 Simulator smoke，不构建或上传签名产物。
- Web、APK 和 AAB 在上传前解包扫描，统一 release bundle 再次解包扫描并校验 checksum。
- 所有自动验证仅使用 fixture、mock、测试数据库、Emulator 或 Simulator，不连接真实节点，不执行真实部署。

## 残余风险

- GitHub runner 镜像、Android Emulator 和 iOS Simulator 可用性属于外部运行环境风险，以本次推送后的 Actions 结果为准。
- Android debug APK 与 unsigned AAB 不是生产分发包；生产签名、商店发布和签名材料管理需另行规划。
