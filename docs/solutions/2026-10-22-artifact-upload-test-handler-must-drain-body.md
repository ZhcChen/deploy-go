# 制品上传测试偶发失败：处理器没读完请求体就回复

## 现象

`agent/tests/artifact_transfer.rs::upload_rejects_put_status_that_jumps_past_the_sent_chunk` 偶发失败：
期望错误类别 `invalid_response`，实际是 `http_rejected`。单独循环运行约 6 次失败 4 次，基线上（没有任何本需求改动）同样出现，
已让 `make api-check` 连续两次变红。生产路径不受影响。

## 根因

测试里的 PUT 处理器 `jumping_upload_chunk` 只读请求头就回复，而客户端发送的块是 1 MiB。
服务端在客户端写完请求体之前就发出响应，随后关闭连接，客户端把它当成**可重试的传输错误**，
进入恢复流程改发 `GET` 状态查询；这个测试路由只注册了 `POST` 与 `PUT`，`GET` 返回 405，
于是错误被归类为 `http_rejected`。是否触发取决于响应与请求体写入的先后，所以是偶发的。

## 修复

测试处理器先把请求体读完（`axum::body::Bytes`）再回复，`stalled_upload_chunk` 同样处理。仅改测试夹具。
修复后连续 30 次 0 失败；撤掉修复 10 次失败 6 次，因果成立。

## 教训

- 测试服务端处理器模拟“服务端拒绝/异常响应”时，要先读完请求体；对大请求体提前回复会让客户端走到传输错误与重试分支，
  测到的就不是想测的那条路径。
- 偶发失败要先拿到真实错误值（这里是 `left: "http_rejected"`）再推断流程，不要猜时序；定位后用“撤掉修复”验证因果。
