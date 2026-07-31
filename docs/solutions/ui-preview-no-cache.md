---
date: 2026-07-31
topic: ui-preview-no-cache
plan: docs/plans/2026-07-31-ui-completion.md
---

# 静态 UI 预览禁用缓存

## 问题

直接使用默认静态服务器时，浏览器可能复用旧的 HTML、CSS 或 JavaScript，导致文件已经修改但预览没有变化，容易误判为实现或选择器未生效。

## 结论

- 统一通过 `make ui` 启动 `ui/serve.py`，默认监听 `127.0.0.1:8050`。
- 所有预览资源返回 `Cache-Control: no-store, no-cache, must-revalidate`，并附带兼容性的 `Pragma` 和 `Expires` 响应头。
- 预览 URL 保持为 `http://127.0.0.1:8050/#/entry`，不使用查询版本号或手工修改资源地址。
- 若页面与源码不一致，先检查响应头、当前服务进程和浏览器实际加载的资源，再排查 CSS 或 JavaScript。

## 验证

```bash
make ui
curl -I http://127.0.0.1:8050/assets/app.js
```

响应中应包含 `Cache-Control: no-store, no-cache, must-revalidate`。
