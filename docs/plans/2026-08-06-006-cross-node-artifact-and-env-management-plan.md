---
title: 跨节点制品中转与业务应用 Env 管理计划
date: 2026-08-06
artifact_contract: ce-unified-plan/v1
artifact_readiness: requirements-only
product_contract_source: ce-brainstorm
execution: code
---

# 跨节点制品中转与业务应用 Env 管理计划

## Goal Capsule

- **目标：** 让 Build Agent 可以把业务应用发布物安全中转给多个 Target Agent，并让管理员在 Web 中查看、编辑和立即同步由业务应用首次上传登记的 Env 文件。
- **应用模型：** 一个 Deploy Go 应用代表一个独立业务环境实例，例如 `qfy-voucher-hub-production` 与 `qfy-voucher-hub-test`；名称后缀只是命名规范，不增加 `environment_kind`。
- **关键边界：** WSS 只承载控制消息，不传发布物；发布物是短期临时数据，Env 是加密、版本化的长期配置；Deploy Go 只调度脚本和同步配置，不接管 Compose 或业务发布逻辑。
- **当前限制：** 现有两阶段部署要求 Build Agent 与 Target Agent 位于同一节点，通过本地 staging 交接。本计划完成前，不得把不同 Agent 配置成已支持的跨节点发布链路。
- **安全原则：** Env 明文只对重新验证身份的管理员按需展示；日志、审计、任务载荷、URL 和 WSS 消息不得包含 Env 内容或制品凭证。

---

## Product Contract

### Summary

业务应用继续提供固定的 `deploy-go-prepare` 和 `deploy-go-release` Make target。Build Agent 检出确定 commit、完成编译打包并校验 manifest 后，通过 HTTPS 把发布物上传到 Deploy Go 临时制品区。主控只保存有期限的部署制品，向每个 Target Agent 签发绑定任务的短期下载凭证；Target Agent 通过 HTTPS 下载、再次校验后执行发布脚本。WSS 只负责下发任务、短期凭证引用、状态、日志和结果。

业务应用可以在准备阶段首次提交 Env 文件清单和初始内容。只有已被业务应用上传登记的 Env 才会出现在 Web；Web 不允许凭空新建 Env 文件。同名文件后续再次上传只确认声明，不覆盖管理员已经维护的值。本次部署未上传已有 Env 时继续保留，只有管理员明确删除才移除。管理员可以在 Web 通过键值表格或黑色终端风格原文编辑器查看和编辑明文；保存后主控立即让所有绑定该应用的在线 Target Agent 原子同步最新版本，离线节点上线后自动补齐。

### Problem Frame

当前 prepare 和 release 使用相同的任务 staging 路径，发布物不经过主控，因此只有同一 Agent 节点能够完整执行两阶段部署。虽然应用来源已经保存 Build Agent、部署目标也绑定 Target Agent，但不同节点之间没有文件传输协议，选择不同 Agent 会让 release 阶段找不到 prepare 产物。

当前部署目标还要求填写 `environment`，但产品已经采用“一个应用代表一个业务环境实例”的模型。同一个业务系统的正式和测试环境应创建为两个应用，因此目标级环境输入重复且容易产生冲突。

当前 Env 只支持管理员预先在节点创建文件，再将绝对路径作为敏感文件引用传给脚本。平台没有 Env 对象、加密版本、Web 编辑、节点同步状态，也不能保证同一应用的多个目标节点使用一致配置。

### Actors

- A1. **管理员：** 创建环境实例应用，管理 Git 来源、Build Agent、目标节点、Env 明文和同步状态。
- A2. **普通用户：** 在授权应用范围内预览和执行部署，不能查看或编辑 Env 明文。
- A3. **Build Agent：** 检出固化 commit、运行 prepare、校验制品和首次登记的 Env，并通过 HTTPS 上传。
- A4. **Target Agent：** 下载并校验制品、同步 Env、运行 release、回传进度和结果。
- A5. **主控：** 保存部署与配置事实、临时中转制品、加密 Env、签发短期凭证、调度任务并记录审计。

### Requirements

**应用与目标模型**

- R1. 一个 Deploy Go 应用必须代表一个独立业务环境实例；同一业务系统的正式、测试和预发布环境分别创建应用。
- R2. 应用 Slug 使用显式环境后缀，例如 `qfy-voucher-hub-production`、`qfy-voucher-hub-test`；后缀只作为命名规范，系统不从 Slug 推断环境行为。
- R3. 不增加 `environment_kind`。部署目标不再要求管理员填写环境，部署上下文使用应用身份和兼容字段，不允许目标自由定义第二套环境事实。
- R4. 一个应用可以绑定多个 Target Agent；同一应用的 Env 内容对所有绑定目标保持一致，不提供节点级覆盖。

**跨节点制品中转**

- R5. Build Agent 与 Target Agent 可以是不同节点，所有 Agent 只需主动连接 Deploy Go，不要求目标节点开放入站文件服务。
- R6. Build Agent 必须先在本地完成 manifest、路径、数量、大小和 SHA-256 校验，再通过 HTTPS 上传制品；业务 Make target 不得自行 SSH、SCP 或调用任意目标地址。
- R7. 主控必须把制品存放在任务隔离的临时区域，设置单文件、总大小、文件数量、上传时间和保留期限上限，不把制品存入数据库或日志。
- R8. 主控只向当前部署的目标任务签发短期、单用途下载凭证；凭证绑定部署、目标 Agent、制品摘要和期限，不能跨任务复用。
- R9. Target Agent 下载完成后必须再次校验 manifest、文件集合、大小和 SHA-256，全部制品完整落盘前不得执行 release。
- R10. WSS 控制协议只传任务和不透明凭证引用，不承载制品字节；上传、下载和断点重试使用 HTTPS 数据通道。
- R11. 同一部署向多个目标节点发布时，每个目标拥有独立下载、校验、执行和终态；单个节点失败不能伪造其他节点成功，也不能静默重复发布。
- R12. 制品在全部目标进入终态或达到保留期限后清理。平台不提供长期制品仓库、历史下载或基于历史制品的回退。

**业务 Env 登记**

- R13. 业务仓库使用稳定文件名声明模块配置，推荐 `compose.env`、`shared.env`、`api.env`、`worker.env`，并在仓库保留对应 `.env.example`。
- R14. 真实 Env 只有通过 Build Agent 的受控上传清单才能首次创建；Web 不提供“新建 Env 文件”操作，未上传的文件不形成可编辑对象。
- R15. Env 清单必须包含稳定文件名、模块、内容摘要和格式版本，拒绝绝对路径、目录穿越、符号链接、重复名称、非法文件名和超限内容。
- R16. 首次上传创建 Env 对象并保存初始内容；后续同名上传只确认业务应用仍声明该文件，不覆盖 Web 当前值。
- R17. 某次部署未上传已有 Env 时，主控继续保留该对象和服务器文件，不自动停用、删除或覆盖。
- R18. Env 删除只能由管理员在 Web 明确执行，并经过影响范围确认；删除和节点清理必须可审计，不与普通部署隐式绑定。

**Env 加密、查看与编辑**

- R19. Env 内容由主控使用现有主密钥体系加密保存；列表、部署快照、任务 JSON、审计、日志和错误响应不得包含明文。
- R20. 只有管理员可以查看和编辑 Env。每次开始明文会话必须重新验证管理员密码，不能仅依赖已有 Cookie。
- R21. Env 列表只返回名称、摘要、版本、更新时间和同步汇总；单个明文读取接口必须禁止浏览器和代理缓存，并在短时编辑会话失效后拒绝继续读取。
- R22. Web 默认提供结构化键值表格，支持增加、修改、删除和校验键；同时提供黑色终端风格原文编辑器，使用等宽字体、行号、dotenv 高亮和错误定位。
- R23. 结构化模式与原文模式编辑同一个文档。模式切换和保存必须通过解析校验，保留可支持的注释与顺序，并拒绝重复键、非法变量名、控制字符和未闭合引号。
- R24. 保存前展示脱敏 Diff 和影响节点数量；Env 值不进入 URL、本地持久缓存、前端遥测或通用错误对象，离开编辑页后清除内存中的明文状态。
- R25. Env 更新使用乐观并发版本。管理员基于旧版本保存时必须显示冲突并重新加载，不能覆盖他人修改。

**Env 节点同步**

- R26. Env 保存成功后立即创建新版本，并为该应用绑定的所有 Target Agent 建立独立同步状态：`pending`、`syncing`、`succeeded` 或 `failed`。
- R27. 主控通过绑定 Env 版本和目标 Agent 的短期 secret lease 传递内容；WSS 任务和持久化 Agent journal 只保存 lease ID、目标路径、版本和摘要。
- R28. Target Agent 只能把 Env 写入自身受控 `secrets_root/<app-slug>/<file-name>`，不得接受主控提供的任意绝对路径。
- R29. Agent 使用同目录 `0600` 临时文件完成校验、fsync 和原子替换；失败时保留上一版本，不能留下半写文件。
- R30. 在线节点立即同步；离线节点保持 `pending`，恢复连接后自动取得并同步最新版本，不补写已经被后续版本取代的旧版本。
- R31. 某一节点同步失败不回滚其他已成功节点。Web 必须展示每个节点的实际版本、最后尝试时间和脱敏错误，并支持管理员重试。

**业务脚本与 Compose**

- R32. Deploy Go 不解析、不生成也不接管 `compose.yaml`；Compose 文件和 `deploy-go-release` 继续由业务仓库维护和审查。
- R33. 业务 release 脚本通过平台注入的固定 Env 文件路径调用 Compose。`compose.env` 用于 Compose 插值，`shared.env` 和模块 Env 通过服务的 `env_file` 进入容器。
- R34. Env 同步成功是 release 的前置条件。目标节点缺少必需 Env、版本落后或同步失败时不得启动业务发布脚本。
- R35. Agent 默认不获得 Docker/root 权限。需要 Compose 特权操作时继续使用应用专属受控 launcher，不把 Env 内容、任意 Docker 参数或 shell 放入 launcher 输入。

### Key Flows

- F1. **跨节点构建与发布**
  - **Trigger:** 用户确认一次应用部署。
  - **Steps:** 主控固化 commit；Build Agent prepare 并校验；通过 HTTPS 上传临时制品；主控创建各目标下载任务；Target Agent 下载并复验；确认 Env 已同步；执行 release；分别回传结果。
  - **Outcome:** Build Agent 与多个 Target Agent 无需共享磁盘即可完成可追溯发布。
  - **Covered by:** R5-R12、R34-R35。
- F2. **首次登记 Env**
  - **Trigger:** Build Agent 首次上传业务仓库声明的 Env 清单和内容。
  - **Steps:** 校验名称、格式、摘要和限额；主控创建加密 Env 对象；创建所有绑定目标的同步记录；在线 Agent 原子写入。
  - **Outcome:** 只有业务应用明确提供的 Env 成为 Web 可管理对象。
  - **Covered by:** R13-R18、R26-R30。
- F3. **管理员编辑 Env**
  - **Trigger:** 管理员从应用 Env 页面选择已有文件。
  - **Steps:** 重新验证密码；按需解密；结构化或原文编辑；校验并查看 Diff；保存新版本；立即同步所有目标；查看逐节点结果。
  - **Outcome:** 主控保存唯一权威版本，节点最终收敛到相同内容。
  - **Covered by:** R19-R31。
- F4. **后续部署重复声明**
  - **Trigger:** 新版本业务代码再次上传同名 Env，或本次未包含旧 Env。
  - **Steps:** 同名只更新声明事实、不覆盖内容；缺席文件保持原对象；管理员修改与节点版本不受部署上传影响。
  - **Outcome:** 代码部署不会意外覆盖或删除生产配置。
  - **Covered by:** R16-R18。

### Acceptance Examples

- AE1. 应用 `qfy-voucher-hub-production` 选择 Build Agent A 和 Target Agent B/C；A 上传制品后，B/C 分别下载并校验，WSS 消息中不出现制品字节。
- AE2. B 下载成功而 C 离线时，B 可以继续发布，C 保持等待；C 上线后取得仍有效的当前部署任务或进入明确终态，不能伪造成已发布。
- AE3. Build Agent 首次上传 `api.env` 后，Web 出现该对象；未上传 `worker.env` 时，Web 不提供创建 `worker.env` 的入口。
- AE4. 管理员在 Web 修改 `api.env` 后，下一次部署再次上传同名文件，主控保留管理员版本，不用仓库上传内容覆盖。
- AE5. 新版本不再上传已有 `shared.env`，对象和所有节点文件保持不变；只有管理员明确删除才能清理。
- AE6. 普通用户访问 Env 接口得到 403；管理员未重新验证密码时只能查看元数据，不能取得明文。
- AE7. 管理员保存 Env 时一台节点离线，在线节点显示 `succeeded`，离线节点显示 `pending`；离线节点恢复后自动同步最新版本。
- AE8. Agent 写入 Env 中途失败，服务器继续保留旧文件，主控显示该节点 `failed`，其他节点不回滚。
- AE9. Env 原文包含重复键或非法变量名时，Web 指向具体行并禁止保存；错误响应和审计不包含对应值。
- AE10. Target Agent 下载的制品与 manifest SHA-256 不一致时，在运行 `deploy-go-release` 前失败，线上服务和 Env 均不变化。

### Scope Boundaries

**本期交付**

- 主控临时制品中转、Build Agent HTTPS 上传、Target Agent HTTPS 下载与复验。
- 应用 Env 首次登记、加密版本、管理员明文编辑、立即多节点同步和状态展示。
- 应用即环境实例，部署目标移除环境输入，不增加 `environment_kind`。
- Web 键值表格与黑色终端风格原文编辑器。

**Deferred for later**

- S3/OSS 等外部对象存储后端、跨主控集群复制、长期制品保留和 CDN。
- 节点级 Env 覆盖、共享基础 Env 继承、变量引用、自动密钥轮换和审批流。
- Web 新建 Env、从服务器反向发现任意文件、非 dotenv 配置编辑和 Compose 可视化编排。

**Outside this product's identity**

- 通用 CI pipeline、任意文件管理器、远程终端、源码托管、Compose/Kubernetes 接管和业务应用自动迁移推导。

### Open Questions For Planning

- OQ1. 临时制品默认单文件、总大小、文件数量、上传超时和保留期限的具体上限。
- OQ2. 多 Target 部署的产品终态：全部成功才算整体成功，还是允许明确的部分成功状态。
- OQ3. 管理员重新验证会话的有效时长，以及删除 Env 是否要求再次验证。
- OQ4. dotenv 首版允许的语法子集，以及注释、空行、引号和多行值的保真范围。
- OQ5. 从现有 `deployment_targets.environment` 迁移到应用实例模型时，历史数据、快照和 API 兼容期的处理方式。

### Sources

- `docs/standards/application-deployment-contract.md`
- `docs/standards/agent-control-protocol.md`
- `docs/standards/git-branch-deployment-contract.md`
- `docs/standards/deploy-script-contract.md`
- `docs/standards/privileged-release-launcher.md`
- `docs/plans/2026-08-06-004-git-branch-two-stage-deployment-plan.md`
