import { FileCode2, Layers, Plus } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { applicationTemplates } from "./applicationTemplates";

export function ApplicationTemplatesPage() {
  const auth = useAuth();
  const isAdministrator = auth.user?.identity === "administrator";
  const [templateId, setTemplateId] = useState(applicationTemplates[0]?.id ?? "");
  const [filePath, setFilePath] = useState(applicationTemplates[0]?.files[0]?.path ?? "");
  const template = applicationTemplates.find((item) => item.id === templateId) ?? applicationTemplates[0];
  const activeFile = template?.files.find((file) => file.path === filePath) ?? template?.files[0];

  return <section className="workspace template-page">
    <div className="workspace-heading"><div><h2>应用模板</h2><p>预设业务仓库骨架，只读查看 Compose、Env 字段与应用配置；正式部署前请复制到独立业务仓库审查。</p></div>{isAdministrator ? <Link className="button button--primary" to={`/templates/new?template=${template?.id ?? ""}`}><Plus aria-hidden="true" />从模板创建应用</Link> : null}</div>
    <div className="template-selector" role="tablist" aria-label="选择应用模板">
      {applicationTemplates.map((item) => <button key={item.id} type="button" role="tab" aria-selected={template?.id === item.id} onClick={() => { setTemplateId(item.id); setFilePath(item.files[0].path); }}><Layers aria-hidden="true" /><span><strong>{item.name}</strong><small>{item.summary}</small></span></button>)}
    </div>
    {template && activeFile ? <>
      <div className="template-file-tabs" role="tablist" aria-label={`${template.name} 配置文件`}>
        {template.files.map((file) => <button key={file.path} type="button" role="tab" aria-selected={activeFile.path === file.path} onClick={() => setFilePath(file.path)}><FileCode2 aria-hidden="true" />{file.label}</button>)}
      </div>
      <section className="template-file-viewer">
        <div className="section-heading"><div><h3>{template.name} · <code>{activeFile.path}</code></h3><p>文件为只读示例；compose.env 与服务级 Env 的正式值通过应用配置加密登记。</p></div></div>
        <pre className="template-file-preview" data-testid="template-file-content">{activeFile.content}</pre>
      </section>
    </> : null}
  </section>;
}
